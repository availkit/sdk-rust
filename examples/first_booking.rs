//! Complete deliberate test lifecycle. A durable latch refuses every rerun.
use availkit_sdk::{models::*, *};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    workspace_id: String,
    credential: String,
    environment: String,
    offering_id: String,
    from: String,
    to: String,
    intent_directory: String,
    base_url: Option<String>,
}
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let path = std::env::var("AVAILKIT_CONFIG_FILE").map_err(|_| input_error())?;
    run_config(&path).await
}
async fn run_config(path: &str) -> Result<()> {
    // docs:configuration:start
    // Bounded owner-only regular config, no symlink or command-line credential.
    let config: Config =
        serde_json::from_slice(&private_files::read(path, 16384)?).map_err(|_| input_error())?;
    let environment = match config.environment.as_str() {
        "test" => Environment::Test,
        _ => return Err(input_error()),
    };
    let client = Client::new(
        &config.workspace_id,
        environment,
        &config.credential,
        Options {
            base_url: config
                .base_url
                .unwrap_or_else(|| "https://api.availkit.com".into()),
            ..Default::default()
        },
    )?;
    let cancellation = CancellationToken::new();
    let directory = std::path::Path::new(&config.intent_directory);
    // Refuse EVERY rerun, including after loss or successful completion. Inspect
    // retained intents/receipt manually; never unlink a latch to retry unknown work.
    private_files::create(
        directory.join("lifecycle-started.json"),
        b"{\"version\":1,\"state\":\"started\"}",
    )?;
    // docs:configuration:end
    // docs:lifecycle:start
    let times = client
        .availability(
            &cancellation,
            &AvailabilityQuery {
                offering_id: config.offering_id.clone(),
                from: config.from,
                to: config.to,
            },
        )
        .await?;
    if times.slots.len() < 2 {
        return Err(input_error());
    }
    let first = &times.slots[0];
    let second = &times.slots[1];
    let intent = client.prepare_book(
        &BookingInput {
            offering_id: config.offering_id,
            start: first.start.clone(),
            customer_id: None,
        },
        &new_idempotency_key()?,
    )?;
    intent.save_private(directory.join("create.json"))?;
    let booking = client.execute(&intent, &cancellation).await?;
    client.get_booking(&cancellation, &booking.id).await?;
    let move_intent = client.prepare_reschedule_booking(
        &booking.id,
        &RescheduleInput {
            expected_start: booking.start.clone(),
            start: second.start.clone(),
        },
        &new_idempotency_key()?,
    )?;
    move_intent.save_private(directory.join("move.json"))?;
    let moved = client.execute(&move_intent, &cancellation).await?;
    let cancel = client.prepare_cancel(&moved.id, &new_idempotency_key()?)?;
    cancel.save_private(directory.join("cancel.json"))?;
    client.execute(&cancel, &cancellation).await?;
    // Keep the durable latch and every original even after success.
    private_files::create(
        directory.join("completed.json"),
        b"{\"version\":1,\"state\":\"completed\"}",
    )?;
    // docs:lifecycle:end
    println!("Test booking created, read, moved and cancelled; private intents retained.");
    Ok(())
}
fn input_error() -> Error {
    Error {
        kind: "input",
        code: "invalid_example_configuration".into(),
        status: None,
        mutation_outcome_unknown: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        os::unix::fs::PermissionsExt,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };
    const ID: &str = "11111111111111111111111111111111";
    const TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    fn directory() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "availkit-rust-example-{}",
            new_idempotency_key().unwrap()
        ));
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
    fn booking(start: &str, status: &str) -> Value {
        json!({"id":ID,"offeringId":ID,"resourceId":ID,"start":start,"end":if start.contains("10:00"){"2030-01-01T10:30:00Z"}else{"2030-01-01T11:30:00Z"},"status":status})
    }
    fn fixture(
        root: &std::path::Path,
        lost: bool,
    ) -> (String, Arc<Mutex<usize>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap();
        let count = Arc::new(Mutex::new(0));
        let observed = count.clone();
        let root = root.to_owned();
        let join = thread::spawn(move || {
            // A fresh process may spend time initializing the platform TLS trust
            // verifier before the example creates its durable latch. Start the
            // network fixture budget only once that initialization has completed.
            let setup_deadline = Instant::now() + Duration::from_secs(20);
            while !root.join("lifecycle-started.json").is_file() {
                assert!(
                    Instant::now() < setup_deadline,
                    "fixture setup did not finish"
                );
                thread::sleep(Duration::from_millis(3));
            }
            for step in 0..if lost { 2 } else { 5 } {
                let deadline = Instant::now() + Duration::from_secs(12);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(3))
                        }
                        Err(e) => panic!("fixture accept failed: {e}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut raw = Vec::new();
                loop {
                    let mut bytes = [0; 4096];
                    let n = stream.read(&mut bytes).unwrap();
                    raw.extend_from_slice(&bytes[..n]);
                    if let Some(pos) = raw.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&raw[..pos]);
                        let length = headers
                            .lines()
                            .find_map(|l| {
                                l.to_lowercase()
                                    .strip_prefix("content-length: ")
                                    .and_then(|v| v.parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if raw.len() >= pos + 4 + length {
                            break;
                        }
                    }
                }
                *observed.lock().unwrap() += 1;
                assert!(root.join("lifecycle-started.json").is_file());
                if step >= 1 {
                    assert!(root.join("create.json").is_file());
                }
                if step >= 3 {
                    assert!(root.join("move.json").is_file());
                }
                if step == 4 {
                    assert!(root.join("cancel.json").is_file());
                }
                if lost && step == 1 {
                    continue;
                }
                let body=match step{0=>json!({"slots":[{"start":"2030-01-01T10:00:00Z","end":"2030-01-01T10:30:00Z","remainingCapacity":1},{"start":"2030-01-01T11:00:00Z","end":"2030-01-01T11:30:00Z","remainingCapacity":1}]}),1|2=>booking("2030-01-01T10:00:00Z","confirmed"),3=>booking("2030-01-01T11:00:00Z","confirmed"),_=>booking("2030-01-01T11:00:00Z","cancelled")}.to_string();
                let status = if step == 1 { 201 } else { 200 };
                write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        (format!("http://{port}"), count, join)
    }
    fn config(root: &std::path::Path, url: String) -> std::path::PathBuf {
        let p = root.join("config.json");
        private_files::create(&p,&serde_json::to_vec(&json!({"workspaceId":ID,"environment":"test","credential":TOKEN,"offeringId":ID,"from":"2030-01-01T09:00:00Z","to":"2030-01-01T12:00:00Z","intentDirectory":root.to_str().unwrap(),"baseUrl":url})).unwrap()).unwrap();
        p
    }
    #[tokio::test]
    async fn actual_example_unknown_create_persists_before_send_stops_then_refuses_rerun() {
        let root = directory();
        let (url, count, join) = fixture(&root, true);
        let path = config(&root, url);
        let error = run_config(path.to_str().unwrap()).await.unwrap_err();
        assert!(error.mutation_outcome_unknown);
        join.join().unwrap();
        assert_eq!(*count.lock().unwrap(), 2);
        assert!(root.join("create.json").is_file());
        assert!(!root.join("move.json").exists());
        assert!(!root.join("cancel.json").exists());
        assert!(run_config(path.to_str().unwrap()).await.is_err());
        assert_eq!(*count.lock().unwrap(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn actual_example_success_moves_then_cancels_and_retains_permanent_latch() {
        let root = directory();
        let (url, count, join) = fixture(&root, false);
        let path = config(&root, url);
        run_config(path.to_str().unwrap()).await.unwrap();
        join.join().unwrap();
        assert_eq!(*count.lock().unwrap(), 5);
        assert!(root.join("completed.json").is_file());
        assert!(run_config(path.to_str().unwrap()).await.is_err());
        assert_eq!(*count.lock().unwrap(), 5);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn config_read_checks_owner_permissions_type_bounds_and_symlinks() {
        let root = directory();
        let path = root.join("config.json");
        private_files::create(&path, b"{}").unwrap();
        assert!(private_files::read(&path, 1).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(private_files::read(&path, 16384).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(private_files::read(&link, 16384).is_err());
        assert!(private_files::create(&path, b"{}").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
