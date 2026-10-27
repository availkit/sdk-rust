//! Public capability-only test lifecycle. Never accepts a workspace credential.
use availkit_sdk::{models::*, *};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    page_id: String,
    offering_id: String,
    from: String,
    to: String,
    contact: CustomerInput,
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
    let config: Config =
        serde_json::from_slice(&private_files::read(path, 16384)?).map_err(|_| input_error())?;
    let client = PublicClient::new(
        &config.page_id,
        Options {
            base_url: config
                .base_url
                .unwrap_or_else(|| "https://api.availkit.com".into()),
            ..Default::default()
        },
    )?;
    let directory = std::path::Path::new(&config.intent_directory);
    private_files::create(
        directory.join("public-lifecycle-started.json"),
        b"{\"version\":1,\"state\":\"started\"}",
    )?;
    let cancel = CancellationToken::new();
    let page = client.get_page(&cancel).await?;
    if page.environment != PublicPageEnvironment::Test {
        return Err(input_error());
    }
    let mut query = PageQuery::default();
    let mut found = false;
    for _ in 0..100 {
        let page = client.list_offerings(&cancel, &query).await?;
        if page.items.iter().any(|o| o.id == config.offering_id) {
            found = true;
            break;
        }
        match page.next_after {
            Some(cursor) => query.after = Some(cursor),
            None => break,
        }
    }
    if !found {
        return Err(input_error());
    }
    let times = client
        .availability(
            &cancel,
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
    let booking_token = new_guest_token()?;
    let contact_token = new_guest_token()?;
    let create = client.prepare_book(
        &PublicBookingInput {
            offering_id: config.offering_id,
            start: times.slots[0].start.clone(),
            contact: config.contact,
        },
        &booking_token,
        &contact_token,
        &new_idempotency_key()?,
    )?;
    create.save_private(directory.join("public-create.json"))?;
    let created = client.execute(&create, &cancel).await?;
    let booking = client
        .get_booking(&cancel, &created.id, &booking_token)
        .await?;
    let move_intent = client.prepare_reschedule(
        &booking.id,
        &RescheduleInput {
            expected_start: booking.start,
            start: times.slots[1].start.clone(),
        },
        &booking_token,
        &new_idempotency_key()?,
    )?;
    move_intent.save_private(directory.join("public-move.json"))?;
    let moved = client.execute(&move_intent, &cancel).await?;
    let cancellation = client.prepare_cancel(&moved.id, &booking_token, &new_idempotency_key()?)?;
    cancellation.save_private(directory.join("public-cancel.json"))?;
    client.execute(&cancellation, &cancel).await?;
    private_files::create(
        directory.join("public-completed.json"),
        b"{\"version\":1,\"state\":\"completed\"}",
    )?;
    println!("Public test booking created, read, moved and cancelled; private originals retained.");
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
