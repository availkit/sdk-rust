# AvailKit Rust SDK 0.1.1

SDK **0.1.1**. Standalone build and synthetic test instructions are in
[DEVELOPMENT.md](DEVELOPMENT.md). [Versioned release notes](https://github.com/availkit/sdk-rust/releases)
record dated verification and available distribution channels. Source availability
does not imply registry publication or mobile/service acceptance.
Install examples name this SDK version; check the release notes for artifact availability.

The trusted backend `Client` implements the maintained 34 workspace operations;
`PublicClient` implements all 7 public booking operations. `coverage.json` records
the exact method/path/status/schema matrix. No account, passkey, recovery or native
login API is provided. Never embed a workspace credential in a browser/mobile app.

Rust 1.90 or newer, edition2024, Tokio 1.x and a system trust store are required. This
release is verified on Linux x86_64; private file helpers and the runnable example
require Unix permissions. Apple/Windows native consumers are not claimed as tested.
MSRV changes require a minor SDK release; stable API contract updates and security
fixes receive a patch release. Dependency versions are pinned in Cargo.toml/lock.

## Install the hosted crate

This is a conventional `.crate`, not a crates.io publication claim. Download and
verify the hosted checksum, then extract into your own project:

<!-- docs:install:start -->
```sh
curl --fail --proto '=https' -O https://availkit.dev/downloads/availkit-sdk-0.1.1.crate
curl --fail --proto '=https' -O https://availkit.dev/downloads/SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
mkdir -p vendor
tar -xzf availkit-sdk-0.1.1.crate -C vendor
```
<!-- docs:install:end -->

```toml
[dependencies]
availkit-sdk = { path = "vendor/availkit-sdk-0.1.1" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

No private application repository is needed. The registry publication/ownership
plan remains separate from this downloadable distribution.

## First booking

`examples/first_booking.rs` is a complete opt-in create/read/move/cancel lifecycle using
an existing test offering with at least two available times. All writes are intentional. A permanent durable `lifecycle-started.json` latch and fixed `create.json`/`move.json`/`cancel.json` originals refuse every rerun, including after loss or completion. Keep them and reconcile manually; never remove a latch to retry unknown work. Create a 0700 intent directory
and a 0600 JSON config file outside your source tree:

```json
{"workspaceId":"YOUR_32_HEX_WORKSPACE_ID","environment":"test","credential":"YOUR_TEST_READ_BOOK_KEY","offeringId":"YOUR_32_HEX_OFFERING_ID","from":"2026-11-02T09:00:00Z","to":"2026-11-02T17:00:00Z","intentDirectory":"/absolute/private/intents"}
```

<!-- docs:run:start -->
```sh
AVAILKIT_CONFIG_FILE=/absolute/private/config.json cargo run --manifest-path vendor/availkit-sdk-0.1.1/Cargo.toml --example first_booking
```
<!-- docs:run:end -->

The server derives the effective environment from the credential. The constructor
requires explicit workspace/environment intent; an environment label does not
override a key. Read+Book is sufficient for an unlinked booking; linking `customerId`
also requires Manage. Catalog writes need Edit catalog, customer/webhook management
needs Manage, privacy pruning/erasure needs Privacy as described in the API docs.

```rust
use availkit_sdk::{Client, Environment, Options, CancellationToken, PageQuery};
# async fn example() -> availkit_sdk::Result<()> {
let client = Client::new("11111111111111111111111111111111", Environment::Test,
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", Options::default())?;
let cancellation = CancellationToken::new();
let page = client.list_resources(&cancellation, &PageQuery::default()).await?;
// Follow page.next_after explicitly with a bounded caller-controlled page budget.
# Ok(()) }
```

`examples/public_booking.rs` is the complete capability-only public lifecycle:
run `cargo run --example public_booking` with a private config containing `pageId`,
`offeringId`, `from`, `to`, `contact` (`displayName`, optional `email`/`phone`),
`intentDirectory` and optional `baseUrl`. It refuses live pages, follows at most
100 offering pages, generates both private capabilities before admission, persists
fixed original files before create/move/cancel, and permanently refuses a rerun in
the same directory. It accepts no workspace credential. Keep all private originals
on every outcome, including lost create acknowledgments with no known booking ID.

Request structs use idiomatic snake_case fields and serde wire camelCase.
`models` includes catalog, hours, bookings, contact, event, usage, webhook and
maintenance types. Unknown fields and malformed success assertions are rejected.
Only resource/offering/customer list operations accept `PageQuery.q`; leave it
`None` elsewhere. All list methods expose explicit cursor/limit pagination.

## Unknown outcomes and explicit original replay

Prepare an immutable booking intent, persist it privately, then call `execute`.
`book`/`cancel`/`reschedule_booking` conveniences prepare and send once, but durable
applications should retain prepared intents before first admission. The public
client includes exact booking and contact capabilities in its private intent.

```rust
# async fn example(client: &availkit_sdk::Client, body: &availkit_sdk::models::BookingInput) -> availkit_sdk::Result<()> {
let intent = client.prepare_book(body, &availkit_sdk::new_idempotency_key()?)?;
intent.save_private("/absolute/owned0700/create.json")?;
let result = client.execute(&intent, &availkit_sdk::CancellationToken::new()).await;
// Keep intent on every result. No automatic re-execution follows an error.
# Ok(()) }
```

`Error.mutation_outcome_unknown` is true for transport failures, invalid mutation
acknowledgments and server 5xx errors. A timeout/cancellation/lost response may
already have committed. Reads do not settle a lost write acknowledgment and a new
idempotency key can create a second booking. Only an explicit `execute` of the
retained exact intent can use the server's 24h booking replay contract. The client
checks origin/workspace/environment/credential fingerprint or public page binding,
creation time, allowed path/operation, exact body/key and capability headers. Privacy
pruning, receipt expiry or changed authority can invalidate replay earlier; confirm
those conditions rather than inventing a replacement. Other mutations have no
replay guarantee. Loading/importing never sends. Intents are secrets; do not log,
commit, attach to issues or expose them in URLs. Export/import APIs are explicit.

TLS certificate/hostname verification is required. Only loopback permits plain
HTTP fixtures. Every request disables redirects, automatic retries, proxy discovery,
cookies, HTTP/2 and idle connection reuse, has a total DNS-to-body deadline
(default 10 seconds, max 120), caps the response body and accepts identity JSON only.
Pass `CancellationToken` or drop the future to cancel; neither causes retry. Errors
retain only finite redacted codes/status, never raw transport/body/header data.

## Webhooks

Use `verify_webhook(raw_body, signature, signing_secret, current_unix_seconds, 300)`
before JSON decoding. Preserve raw bytes and transactionally deduplicate
`AvailKit-Delivery-ID` only after verification. Returns false for invalid signatures
or stale timestamps; invalid tolerance is an input error. Webhook subscription
creation contains a once-only `signing_secret`; persist it privately immediately.

## Build and tests

```sh
cargo test --locked --all-targets -j1
python3 build.py
python3 tests/package_test.py
cargo package --locked -j1
```

Tests run loopback HTTP sockets, including the complete 41-operation typed matrix, actual example success/lost-response stop/rerun refusal, all 7 public calls, lost acknowledgment
and byte-identical explicit replay, deadline/cancellation, scope/environment/key/age
guards, malformed JSON/redirects, finite errors and exact-body webhook signatures.
Production authenticated workspace lifecycle/isolation acceptance is additional
release evidence, not a claim made by these synthetic fixtures.


See [CHANGELOG.md](CHANGELOG.md), [SUPPORT.md](SUPPORT.md) and
[SECURITY.md](SECURITY.md) for release policy and private reporting prerequisites.
`contract-projection.json` pins only this SDK's supported wire operations;
additive unrelated API routes do not imply this SDK implements them.

The separate optional business export endpoint and current versioned browser
recovery protocol are not methods of this SDK. Use their documented browser/
TypeScript integration where appropriate; this package preserves its declared
workspace/public booking surface rather than inventing account/native APIs.
