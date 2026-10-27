use availkit_sdk::{models::*, *};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
const ID: &str = "11111111111111111111111111111111";
const TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const START: &str = "2026-10-26T10:00:00Z";
type Requests = Arc<Mutex<Vec<String>>>;
fn server(
    replies: Vec<Option<(u16, String, Duration)>>,
) -> (String, Requests, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let record = requests.clone();
    let handle = thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let request = read(&mut stream);
            record.lock().unwrap().push(request);
            if let Some((status, body, delay)) = reply {
                thread::sleep(delay);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        }
    });
    (format!("http://{addr}"), requests, handle)
}
fn read(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    loop {
        let mut b = [0; 4096];
        let n = stream.read(&mut b).unwrap_or(0);
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&b[..n]);
        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            let h = String::from_utf8_lossy(&bytes[..pos]);
            let length = h
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|v| v.parse::<usize>().ok())
                })
                .unwrap_or(0);
            if bytes.len() >= pos + 4 + length {
                break;
            }
        }
    }
    String::from_utf8(bytes).unwrap()
}
fn page() -> Value {
    json!({"id":ID,"title":"Test","environment":"test"})
}
fn offering() -> Value {
    json!({"id":ID,"name":"Meeting","durationMinutes":30,"timeZone":"UTC"})
}
fn booking(status: &str) -> Value {
    json!({"id":ID,"start":START,"end":"2026-10-26T10:30:00Z","status":status,"offering":offering(),"page":page(),"pageEnabled":true})
}
fn options(base_url: String) -> Options {
    Options {
        base_url,
        ..Default::default()
    }
}
fn body() -> PublicBookingInput {
    PublicBookingInput {
        offering_id: ID.into(),
        start: START.into(),
        contact: CustomerInput {
            display_name: "Test".into(),
            email: None,
            phone: None,
        },
    }
}
#[tokio::test]
async fn actual_public_lifecycle_keeps_exact_capabilities_and_intent_bytes() {
    let values = vec![
        (200, page()),
        (200, json!({"items":[offering()],"nextAfter":null})),
        (
            200,
            json!({"slots":[{"start":START,"end":"2026-10-26T10:30:00Z","remainingCapacity":1}]}),
        ),
        (201, booking("confirmed")),
        (200, booking("confirmed")),
        (200, booking("confirmed")),
        (200, booking("cancelled")),
    ];
    let (url, requests, join) = server(
        values
            .into_iter()
            .map(|(s, v)| Some((s, v.to_string(), Duration::ZERO)))
            .collect(),
    );
    let c = PublicClient::new(ID, options(url)).unwrap();
    let cancel = CancellationToken::new();
    c.get_page(&cancel).await.unwrap();
    c.list_offerings(&cancel, &PageQuery::default())
        .await
        .unwrap();
    c.availability(
        &cancel,
        &AvailabilityQuery {
            offering_id: ID.into(),
            from: START.into(),
            to: "2026-10-26T11:00:00Z".into(),
        },
    )
    .await
    .unwrap();
    let intent = c
        .prepare_book(
            &body(),
            TOKEN,
            TOKEN,
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE",
        )
        .unwrap();
    let raw = intent.export_private().unwrap();
    assert!(!format!("{intent:?}").contains(TOKEN));
    let restored = Intent::import_private(&raw).unwrap();
    c.execute(&restored, &cancel).await.unwrap();
    c.get_booking(&cancel, ID, TOKEN).await.unwrap();
    c.reschedule(
        &cancel,
        ID,
        &RescheduleInput {
            expected_start: START.into(),
            start: START.into(),
        },
        TOKEN,
        TOKEN,
    )
    .await
    .unwrap();
    c.cancel(&cancel, ID, TOKEN, TOKEN).await.unwrap();
    join.join().unwrap();
    let req = requests.lock().unwrap();
    assert_eq!(req.len(), 7);
    assert!(
        req[3]
            .to_lowercase()
            .contains(&format!("authorization: booking {TOKEN}").to_lowercase())
    );
    assert!(
        req[3].contains("X-AvailKit-Contact-Token") || req[3].contains("x-availkit-contact-token")
    );
    assert!(
        req[3].contains(
            &serde_json::from_slice::<Value>(&raw).unwrap()["body"]
                .as_str()
                .unwrap()
                .to_string()
        )
    );
    assert!(!req[0].to_lowercase().contains("authorization:"));
    assert!(!req.iter().any(|r| r.to_lowercase().contains("cookie:")));
}
#[tokio::test]
async fn lost_response_has_no_retry_then_explicit_original_replay() {
    let (url, requests, join) = server(vec![
        None,
        Some((201, booking("confirmed").to_string(), Duration::ZERO)),
    ]);
    let c = PublicClient::new(ID, options(url)).unwrap();
    let cancel = CancellationToken::new();
    let i = c.prepare_book(&body(), TOKEN, TOKEN, TOKEN).unwrap();
    let e = c.execute(&i, &cancel).await.unwrap_err();
    assert!(e.mutation_outcome_unknown);
    assert_eq!(requests.lock().unwrap().len(), 1);
    c.execute(&i, &cancel).await.unwrap();
    join.join().unwrap();
    let r = requests.lock().unwrap();
    assert_eq!(r[0], r[1]);
}
#[tokio::test]
async fn api_errors_redact_untrusted_text_and_do_not_retry() {
    let (url, r, j) = server(vec![Some((
        403,
        json!({"error":{"code":"scope_denied","message":TOKEN}}).to_string(),
        Duration::ZERO,
    ))]);
    let c = Client::new(ID, Environment::Test, TOKEN, options(url)).unwrap();
    let e = c.get_usage(&CancellationToken::new()).await.unwrap_err();
    assert_eq!(e.code, "scope_denied");
    assert!(!e.to_string().contains(TOKEN));
    assert!(!e.mutation_outcome_unknown);
    j.join().unwrap();
    assert_eq!(r.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn original_intents_bind_origin_environment_credential_and_age() {
    let c = Client::new(
        ID,
        Environment::Test,
        TOKEN,
        options("http://127.0.0.1:9".into()),
    )
    .unwrap();
    let i = c
        .prepare_book(
            &BookingInput {
                offering_id: ID.into(),
                start: START.into(),
                customer_id: None,
            },
            TOKEN,
        )
        .unwrap();
    let live = Client::new(
        ID,
        Environment::Live,
        TOKEN,
        options("http://127.0.0.1:9".into()),
    )
    .unwrap();
    assert_eq!(
        live.execute(&i, &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        "intent_scope_mismatch"
    );
    let other = Client::new(
        ID,
        Environment::Test,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE",
        options("http://127.0.0.1:9".into()),
    )
    .unwrap();
    assert_eq!(
        other
            .execute(&i, &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        "intent_scope_mismatch"
    );
    let mut raw: Value = serde_json::from_slice(&i.export_private().unwrap()).unwrap();
    raw["createdAtMs"] = json!(1);
    let expired = Intent::import_private(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(
        c.execute(&expired, &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        "intent_expired"
    );
    raw["createdAtMs"] = json!(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 4000
    );
    let slightly_future = Intent::import_private(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(
        c.execute(&slightly_future, &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        "invalid_intent_timestamp"
    );
    raw["createdAtMs"] = json!(u64::MAX);
    let future = Intent::import_private(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(
        c.execute(&future, &CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        "invalid_intent_timestamp"
    );
}
#[tokio::test]
async fn slow_response_total_deadline_and_cancellation_join_without_retry() {
    let (url, r, j) = server(vec![Some((
        200,
        page().to_string(),
        Duration::from_millis(150),
    ))]);
    let c = PublicClient::new(
        ID,
        Options {
            timeout: Duration::from_millis(40),
            ..options(url)
        },
    )
    .unwrap();
    let e = c.get_page(&CancellationToken::new()).await.unwrap_err();
    assert_eq!(e.code, "timeout");
    j.join().unwrap();
    assert_eq!(r.lock().unwrap().len(), 1);
    let (url, r, j) = server(vec![Some((
        200,
        page().to_string(),
        Duration::from_millis(150),
    ))]);
    let c = PublicClient::new(ID, options(url)).unwrap();
    let token = CancellationToken::new();
    let t = token.clone();
    let send = tokio::spawn(async move { c.get_page(&t).await });
    tokio::time::sleep(Duration::from_millis(25)).await;
    token.cancel();
    assert_eq!(send.await.unwrap().unwrap_err().code, "cancelled");
    j.join().unwrap();
    assert_eq!(r.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn malformed_ack_is_unknown_and_redirect_never_followed() {
    let (url, r, j) = server(vec![Some((
        201,
        "{\"id\":1,\"id\":2}".into(),
        Duration::ZERO,
    ))]);
    let c = PublicClient::new(ID, options(url)).unwrap();
    let e = c
        .book(&CancellationToken::new(), &body(), TOKEN, TOKEN, TOKEN)
        .await
        .unwrap_err();
    assert!(e.mutation_outcome_unknown);
    assert_eq!(e.code, "invalid_json");
    j.join().unwrap();
    assert_eq!(r.lock().unwrap().len(), 1);
    let (url, r, j) = server(vec![Some((302, "{}".into(), Duration::ZERO))]);
    let c = PublicClient::new(ID, options(url)).unwrap();
    assert_eq!(
        c.get_page(&CancellationToken::new())
            .await
            .unwrap_err()
            .kind,
        "protocol"
    );
    j.join().unwrap();
    assert_eq!(r.lock().unwrap().len(), 1);
}
#[test]
fn validation_prevents_invalid_inputs_and_redacts() {
    assert!(
        Client::new(
            ID,
            Environment::Test,
            TOKEN,
            options("http://example.com".into())
        )
        .is_err()
    );
    assert!(PublicClient::new("../", options("http://127.0.0.1:9".into())).is_err());
    let c = PublicClient::new(ID, options("http://127.0.0.1:9".into())).unwrap();
    let mut b = body();
    b.offering_id = "../".into();
    assert!(c.prepare_book(&b, TOKEN, TOKEN, TOKEN).is_err());
    assert!(c.prepare_book(&body(), "short", TOKEN, TOKEN).is_err());
    let ws = Client::new(
        ID,
        Environment::Test,
        TOKEN,
        options("http://127.0.0.1:9".into()),
    )
    .unwrap();
    assert!(!format!("{ws:?}").contains(TOKEN));
}
#[test]
fn webhook_raw_bytes_timestamp_and_signature_are_verified() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    let secret = URL_SAFE_NO_PAD.decode(TOKEN).unwrap();
    let mut h = Hmac::<Sha256>::new_from_slice(&secret).unwrap();
    h.update(b"123.{}");
    let digest = h
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let sig = format!("t=123,v1={digest}");
    assert!(verify_webhook(b"{}", &sig, TOKEN, 123, 300).unwrap());
    assert!(!verify_webhook(b"{ }", &sig, TOKEN, 123, 300).unwrap());
    assert!(!verify_webhook(b"{}", &sig, TOKEN, 424, 300).unwrap());
}

#[tokio::test]
async fn all_41_typed_operations_send_the_contract_path_body_status_and_authority() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures.json")).unwrap();
    let specs: Value = serde_json::from_str(include_str!("../src/operations.json")).unwrap();
    {
        let f = &fixtures["createResource"];
        let (url, requests, join) =
            server(vec![Some((201, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: ResourceInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .create_resource(&CancellationToken::new(), &body)
            .await
            .expect("createResource");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["createResource"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["listResources"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .list_resources(&CancellationToken::new(), &PageQuery::default())
            .await
            .expect("listResources");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listResources"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["setHours"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: Hours = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .replace_hours(
                &CancellationToken::new(),
                f["ids"]["resourceId"].as_str().unwrap(),
                &body,
            )
            .await
            .expect("setHours");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["setHours"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["getHours"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_hours(
                &CancellationToken::new(),
                f["ids"]["resourceId"].as_str().unwrap(),
            )
            .await
            .expect("getHours");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getHours"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["createOffering"];
        let (url, requests, join) =
            server(vec![Some((201, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: OfferingInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .create_offering(&CancellationToken::new(), &body)
            .await
            .expect("createOffering");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["createOffering"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["listOfferings"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .list_offerings(&CancellationToken::new(), &PageQuery::default())
            .await
            .expect("listOfferings");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listOfferings"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["availability"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let range = AvailabilityQuery {
            offering_id: f["query"]["offeringId"].as_str().unwrap().into(),
            from: f["query"]["from"].as_str().unwrap().into(),
            to: f["query"]["to"].as_str().unwrap().into(),
        };
        let actual = c
            .availability(&CancellationToken::new(), &range)
            .await
            .expect("availability");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["availability"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["book"];
        let (url, requests, join) =
            server(vec![Some((201, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: BookingInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .book(&CancellationToken::new(), &body, TOKEN)
            .await
            .expect("book");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["book"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["listBookings"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .list_bookings(&CancellationToken::new(), &PageQuery::default())
            .await
            .expect("listBookings");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listBookings"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["cancel"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .cancel(
                &CancellationToken::new(),
                f["ids"]["bookingId"].as_str().unwrap(),
                TOKEN,
            )
            .await
            .expect("cancel");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["cancel"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["events"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .events(&CancellationToken::new(), &EventQuery::default())
            .await
            .expect("events");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["events"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["getResource"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_resource(
                &CancellationToken::new(),
                f["ids"]["resourceId"].as_str().unwrap(),
            )
            .await
            .expect("getResource");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getResource"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["updateResource"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: ResourceInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .replace_resource(
                &CancellationToken::new(),
                f["ids"]["resourceId"].as_str().unwrap(),
                &body,
            )
            .await
            .expect("updateResource");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["updateResource"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["getOffering"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_offering(
                &CancellationToken::new(),
                f["ids"]["offeringId"].as_str().unwrap(),
            )
            .await
            .expect("getOffering");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getOffering"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["updateOffering"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: OfferingInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .replace_offering(
                &CancellationToken::new(),
                f["ids"]["offeringId"].as_str().unwrap(),
                &body,
            )
            .await
            .expect("updateOffering");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["updateOffering"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["getBooking"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_booking(
                &CancellationToken::new(),
                f["ids"]["bookingId"].as_str().unwrap(),
            )
            .await
            .expect("getBooking");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getBooking"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["createWebhook"];
        let (url, requests, join) =
            server(vec![Some((201, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: WebhookCreate = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .create_webhook(&CancellationToken::new(), &body)
            .await
            .expect("createWebhook");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["createWebhook"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["listWebhooks"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .list_webhooks(&CancellationToken::new(), &PageQuery::default())
            .await
            .expect("listWebhooks");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listWebhooks"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["disableWebhook"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .disable_webhook(
                &CancellationToken::new(),
                f["ids"]["subscriptionId"].as_str().unwrap(),
            )
            .await
            .expect("disableWebhook");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["disableWebhook"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["listWebhookDeliveries"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .list_webhook_deliveries(
                &CancellationToken::new(),
                f["ids"]["subscriptionId"].as_str().unwrap(),
                &PageQuery::default(),
            )
            .await
            .expect("listWebhookDeliveries");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listWebhookDeliveries"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["getUsage"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_usage(&CancellationToken::new())
            .await
            .expect("getUsage");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getUsage"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["rescheduleBooking"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: RescheduleInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .reschedule_booking(
                &CancellationToken::new(),
                f["ids"]["bookingId"].as_str().unwrap(),
                &body,
                TOKEN,
            )
            .await
            .expect("rescheduleBooking");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["rescheduleBooking"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["archiveResource"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .archive_resource(
                &CancellationToken::new(),
                f["ids"]["resourceId"].as_str().unwrap(),
            )
            .await
            .expect("archiveResource");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["archiveResource"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["archiveOffering"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .archive_offering(
                &CancellationToken::new(),
                f["ids"]["offeringId"].as_str().unwrap(),
            )
            .await
            .expect("archiveOffering");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["archiveOffering"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["pruneHistory"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: HistoryPruneInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .prune_history(&CancellationToken::new(), &body)
            .await
            .expect("pruneHistory");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["pruneHistory"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["createCustomer"];
        let (url, requests, join) =
            server(vec![Some((201, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: CustomerInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .create_customer(&CancellationToken::new(), &body)
            .await
            .expect("createCustomer");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["createCustomer"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["listCustomers"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .list_customers(&CancellationToken::new(), &PageQuery::default())
            .await
            .expect("listCustomers");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listCustomers"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["getCustomer"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_customer(
                &CancellationToken::new(),
                f["ids"]["customerId"].as_str().unwrap(),
            )
            .await
            .expect("getCustomer");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getCustomer"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["replaceCustomer"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: CustomerInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .replace_customer(
                &CancellationToken::new(),
                f["ids"]["customerId"].as_str().unwrap(),
                &body,
            )
            .await
            .expect("replaceCustomer");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["replaceCustomer"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["archiveCustomer"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .archive_customer(
                &CancellationToken::new(),
                f["ids"]["customerId"].as_str().unwrap(),
            )
            .await
            .expect("archiveCustomer");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["archiveCustomer"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["eraseCustomer"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .erase_customer(
                &CancellationToken::new(),
                f["ids"]["customerId"].as_str().unwrap(),
            )
            .await
            .expect("eraseCustomer");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["eraseCustomer"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["pruneExpiredReplays"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: MaintenanceInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .prune_expired_replays(&CancellationToken::new(), &body)
            .await
            .expect("pruneExpiredReplays");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["pruneExpiredReplays"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["getPublicPage"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let actual = c
            .get_page(&CancellationToken::new())
            .await
            .expect("getPublicPage");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getPublicPage"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(!request.to_lowercase().contains("authorization:"));
    }
    {
        let f = &fixtures["listPublicOfferings"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let actual = c
            .list_offerings(&CancellationToken::new(), &PageQuery::default())
            .await
            .expect("listPublicOfferings");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["listPublicOfferings"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(!request.to_lowercase().contains("authorization:"));
    }
    {
        let f = &fixtures["publicAvailability"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let range = AvailabilityQuery {
            offering_id: f["query"]["offeringId"].as_str().unwrap().into(),
            from: f["query"]["from"].as_str().unwrap().into(),
            to: f["query"]["to"].as_str().unwrap().into(),
        };
        let actual = c
            .availability(&CancellationToken::new(), &range)
            .await
            .expect("publicAvailability");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["publicAvailability"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(!request.to_lowercase().contains("authorization:"));
    }
    {
        let f = &fixtures["bookPublic"];
        let (url, requests, join) =
            server(vec![Some((201, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let body: PublicBookingInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .book(&CancellationToken::new(), &body, TOKEN, TOKEN, TOKEN)
            .await
            .expect("bookPublic");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["bookPublic"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["getPublicBooking"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let actual = c
            .get_booking(
                &CancellationToken::new(),
                f["ids"]["bookingId"].as_str().unwrap(),
                TOKEN,
            )
            .await
            .expect("getPublicBooking");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getPublicBooking"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
    }
    {
        let f = &fixtures["cancelPublicBooking"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let actual = c
            .cancel(
                &CancellationToken::new(),
                f["ids"]["bookingId"].as_str().unwrap(),
                TOKEN,
                TOKEN,
            )
            .await
            .expect("cancelPublicBooking");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["cancelPublicBooking"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["reschedulePublicBooking"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = PublicClient::new(&"a".repeat(32), options(url)).unwrap();
        let body: RescheduleInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .reschedule(
                &CancellationToken::new(),
                f["ids"]["bookingId"].as_str().unwrap(),
                &body,
                TOKEN,
                TOKEN,
            )
            .await
            .expect("reschedulePublicBooking");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["reschedulePublicBooking"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
    {
        let f = &fixtures["getBookingPage"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let actual = c
            .get_booking_page(&CancellationToken::new())
            .await
            .expect("getBookingPage");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["getBookingPage"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
    }
    {
        let f = &fixtures["replaceBookingPage"];
        let (url, requests, join) =
            server(vec![Some((200, f["response"].to_string(), Duration::ZERO))]);
        let c = Client::new(&"a".repeat(32), Environment::Test, TOKEN, options(url)).unwrap();
        let body: BookingPageInput = serde_json::from_value(f["request"].clone()).unwrap();
        let actual = c
            .replace_booking_page(&CancellationToken::new(), &body)
            .await
            .expect("replaceBookingPage");
        assert_eq!(serde_json::to_value(actual).unwrap(), f["response"]);
        join.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let spec = &specs["replaceBookingPage"];
        let mut path = spec["path"]
            .as_str()
            .unwrap()
            .replace("{workspaceId}", &"a".repeat(32))
            .replace("{pageId}", &"a".repeat(32));
        for (k, v) in f["ids"].as_object().unwrap() {
            path = path.replace(&format!("{{{k}}}"), v.as_str().unwrap());
        }
        assert!(request.starts_with(&format!("{} {}", spec["method"].as_str().unwrap(), path)));
        assert!(
            request
                .to_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}").to_lowercase())
        );
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            f["request"]
        );
    }
}

#[tokio::test]
async fn forged_retained_raw_bodies_cannot_be_normalized_into_valid_replays() {
    let c = PublicClient::new(ID, options("http://127.0.0.1:9".into())).unwrap();
    let original = c.prepare_cancel(ID, TOKEN, TOKEN).unwrap();
    let record: Value = serde_json::from_slice(&original.export_private().unwrap()).unwrap();
    for body in ["{\"x\":1,\"x\":2}".to_string(), " ".repeat(16385) + "{}"] {
        let mut value = record.clone();
        value["body"] = json!(body);
        let imported = Intent::import_private(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            c.execute(&imported, &CancellationToken::new())
                .await
                .unwrap_err()
                .code,
            "invalid_intent"
        );
    }
}
#[tokio::test]
async fn unknown_or_malformed_4xx_error_keeps_mutation_outcome_unknown() {
    for (value, code) in [
        (
            json!({"error":{"code":"private-unrecognized","message":TOKEN}}),
            "api_error",
        ),
        (json!({"error":{"code":"scope_denied"}}), "invalid_error"),
    ] {
        let (url, requests, join) = server(vec![Some((400, value.to_string(), Duration::ZERO))]);
        let c = PublicClient::new(ID, options(url)).unwrap();
        let error = c
            .book(&CancellationToken::new(), &body(), TOKEN, TOKEN, TOKEN)
            .await
            .unwrap_err();
        assert!(error.mutation_outcome_unknown);
        assert_eq!(error.code, code);
        assert!(!error.to_string().contains(TOKEN));
        join.join().unwrap();
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
}
#[tokio::test]
async fn malformed_hours_ack_is_protocol_unknown_after_one_actual_put() {
    let (url, requests, join) = server(vec![Some((
        200,
        json!({"windows":[{"weekday":1,"start":"25:00","end":"26:00"}]}).to_string(),
        Duration::ZERO,
    ))]);
    let c = Client::new(ID, Environment::Test, TOKEN, options(url)).unwrap();
    let error = c
        .replace_hours(&CancellationToken::new(), ID, &Hours { windows: vec![] })
        .await
        .unwrap_err();
    assert!(error.mutation_outcome_unknown);
    assert_eq!(error.kind, "protocol");
    join.join().unwrap();
    let records = requests.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].starts_with("PUT "));
}
