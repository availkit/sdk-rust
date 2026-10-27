//! AvailKit stable workspace and public-booking clients. Every call sends once.
//! Persist prepared booking intents before the first send. Unknown outcomes are
//! not failures to commit; inspect the original intent before explicit replay.
mod methods;
pub mod models;
#[cfg(unix)]
pub mod private_files;
mod relations;
mod validation;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fmt,
    marker::PhantomData,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use validation::{capability, identifier, input, validate};
pub type Result<T> = std::result::Result<T, Error>;

/// Finite redacted diagnostics; never retains request, response or transport errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: &'static str,
    pub code: String,
    pub status: Option<u16>,
    pub mutation_outcome_unknown: bool,
}
impl Error {
    fn input(code: &str) -> Self {
        Self {
            kind: "input",
            code: code.into(),
            status: None,
            mutation_outcome_unknown: false,
        }
    }
    fn protocol(code: &str, status: u16) -> Self {
        Self {
            kind: "protocol",
            code: code.into(),
            status: Some(status),
            mutation_outcome_unknown: false,
        }
    }
    fn transport(code: &str) -> Self {
        Self {
            kind: "transport",
            code: code.into(),
            status: None,
            mutation_outcome_unknown: false,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AvailKit {}: {}", self.kind, self.code)
    }
}
impl std::error::Error for Error {}

/// Cooperative cancellation. Dropping a request future also cancels it. Neither retries.
#[derive(Clone, Default, Debug)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    async fn cancelled(&self) {
        while !self.is_cancelled() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Test,
    Live,
}
impl Environment {
    fn label(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Live => "live",
        }
    }
}

/// TLS origins only, with HTTP permitted solely on loopback for local fixtures.
#[derive(Clone, Debug)]
pub struct Options {
    pub base_url: String,
    pub timeout: Duration,
    pub max_response_bytes: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            base_url: "https://api.availkit.com".into(),
            timeout: Duration::from_secs(10),
            max_response_bytes: 1_048_576,
        }
    }
}
#[derive(Clone)]
pub struct Client {
    inner: Inner,
    pub workspace_id: String,
    pub environment: Environment,
}
#[derive(Clone)]
pub struct PublicClient {
    inner: Inner,
    pub page_id: String,
}
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AvailKit Client (credentials redacted)")
    }
}
impl fmt::Debug for PublicClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AvailKit PublicClient")
    }
}
#[derive(Clone)]
struct Inner {
    http: reqwest::Client,
    origin: String,
    root: String,
    scope: String,
    credential: Option<String>,
    public: bool,
    timeout: Duration,
    max: usize,
}
impl Client {
    /// Environment records caller intent; only the issued key determines authority.
    pub fn new(
        workspace_id: &str,
        environment: Environment,
        credential: &str,
        options: Options,
    ) -> Result<Self> {
        identifier(workspace_id)?;
        capability(credential)?;
        let inner = Inner::new(
            format!("/v1/workspaces/{workspace_id}"),
            false,
            Some(credential.into()),
            options,
            &format!(
                "{workspace_id}:{}:{}",
                environment.label(),
                hex(&Sha256::digest(credential))
            ),
        )?;
        Ok(Self {
            inner,
            workspace_id: workspace_id.into(),
            environment,
        })
    }
    pub async fn execute(
        &self,
        intent: &Intent<models::Booking>,
        cancel: &CancellationToken,
    ) -> Result<models::Booking> {
        self.inner.execute(intent, cancel).await
    }
}
impl PublicClient {
    /// Public page/capability path only: no native account or workspace credential APIs.
    pub fn new(page_id: &str, options: Options) -> Result<Self> {
        identifier(page_id)?;
        Ok(Self {
            inner: Inner::new(
                format!("/v1/public/pages/{page_id}"),
                true,
                None,
                options,
                &format!("public:{page_id}"),
            )?,
            page_id: page_id.into(),
        })
    }
    pub async fn execute(
        &self,
        intent: &Intent<models::PublicBooking>,
        cancel: &CancellationToken,
    ) -> Result<models::PublicBooking> {
        self.inner.execute(intent, cancel).await
    }
}
fn operation(name: &str) -> Value {
    static OPS: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        serde_json::from_str(include_str!("operations.json")).expect("bundled operations")
    })[name]
        .clone()
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub(crate) fn hex(v: &[u8]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn new_idempotency_key() -> Result<String> {
    random_token(24)
}
pub fn new_guest_token() -> Result<String> {
    random_token(32)
}
fn random_token(size: usize) -> Result<String> {
    let mut bytes = vec![0u8; size];
    getrandom::fill(&mut bytes).map_err(|_| Error::input("entropy_unavailable"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn key(v: &str) -> Result<()> {
    if !(16..=128).contains(&v.len())
        || v.trim() != v
        || !v.bytes().all(|b| (32..=126).contains(&b))
    {
        Err(Error::input("invalid_idempotency_key"))
    } else {
        Ok(())
    }
}
fn origin(value: &str) -> Result<String> {
    let u = reqwest::Url::parse(value).map_err(|_| Error::input("invalid_base_url"))?;
    let local = matches!(
        u.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    );
    if !value.bytes().all(|c| c > 32 && c < 127)
        || !matches!(u.scheme(), "https" | "http")
        || u.scheme() == "http" && !local
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
    {
        return Err(Error::input("invalid_base_url"));
    }
    Ok(u.as_str().trim_end_matches('/').into())
}
impl Inner {
    fn new(
        root: String,
        public: bool,
        credential: Option<String>,
        options: Options,
        scope: &str,
    ) -> Result<Self> {
        if options.timeout.is_zero()
            || options.timeout > Duration::from_secs(120)
            || !(1..=8_388_608).contains(&options.max_response_bytes)
        {
            return Err(Error::input("invalid_limits"));
        }
        let origin = origin(&options.base_url)?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .http1_only()
            .pool_max_idle_per_host(0)
            .timeout(options.timeout)
            .user_agent("availkit-rust/0.1.1")
            .build()
            .map_err(|_| Error::input("transport_configuration"))?;
        Ok(Self {
            http,
            root,
            scope: format!("{origin}:{scope}"),
            origin,
            credential,
            public,
            timeout: options.timeout,
            max: options.max_response_bytes,
        })
    }
    fn prepare<T>(
        &self,
        op: &str,
        path: String,
        body: Value,
        idempotency_key: &str,
        headers: Vec<(String, String)>,
    ) -> Result<Intent<T>> {
        key(idempotency_key)?;
        let bytes = input(&body, &operation(op)["input"])?;
        Ok(Intent {
            record: IntentRecord {
                version: 1,
                operation: op.into(),
                path,
                body: String::from_utf8(bytes).map_err(|_| Error::input("invalid_json"))?,
                idempotency_key: idempotency_key.into(),
                scope: self.scope.clone(),
                headers,
                created_at_ms: now_ms(),
            },
            marker: PhantomData,
        })
    }
    async fn execute<T: DeserializeOwned>(
        &self,
        intent: &Intent<T>,
        cancel: &CancellationToken,
    ) -> Result<T> {
        let r = &intent.record;
        let op = r.operation.as_str();
        let pubop = matches!(
            op,
            "bookPublic" | "cancelPublicBooking" | "reschedulePublicBooking"
        );
        if r.version != 1
            || r.scope != self.scope
            || pubop != self.public
            || !matches!(
                op,
                "book"
                    | "cancel"
                    | "rescheduleBooking"
                    | "bookPublic"
                    | "cancelPublicBooking"
                    | "reschedulePublicBooking"
            )
        {
            return Err(Error::input("intent_scope_mismatch"));
        }
        let age = now_ms() as i128 - r.created_at_ms as i128;
        if age >= 86_400_000 {
            return Err(Error::input("intent_expired"));
        }
        if age < 0 {
            return Err(Error::input("invalid_intent_timestamp"));
        }
        key(&r.idempotency_key)?;
        let spec = operation(op);
        let template = spec["path"].as_str().unwrap().split_once('}').unwrap().1;
        let expected = if template.contains("{bookingId}") {
            let parts: Vec<_> = r.path.split('/').collect();
            if parts.len() != 4 {
                return Err(Error::input("invalid_intent"));
            }
            identifier(parts[2])?;
            template.replace("{bookingId}", parts[2])
        } else {
            template.into()
        };
        if expected != r.path {
            return Err(Error::input("invalid_intent"));
        }
        if r.body.len() > 16384 {
            return Err(Error::input("invalid_intent"));
        }
        let body: Value = validation::strict_json(r.body.as_bytes())
            .map_err(|_| Error::input("invalid_intent"))?;
        input(&body, &spec["input"])?;
        if r.headers.len()
            != if pubop {
                if op == "bookPublic" { 2 } else { 1 }
            } else {
                0
            }
        {
            return Err(Error::input("invalid_intent"));
        }
        if pubop {
            let h = r
                .headers
                .iter()
                .find(|(k, _)| k == "Authorization")
                .ok_or_else(|| Error::input("invalid_intent"))?;
            capability(
                h.1.strip_prefix("Booking ")
                    .ok_or_else(|| Error::input("invalid_intent"))?,
            )?;
            if op == "bookPublic" {
                capability(
                    &r.headers
                        .iter()
                        .find(|(k, _)| k == "X-AvailKit-Contact-Token")
                        .ok_or_else(|| Error::input("invalid_intent"))?
                        .1,
                )?;
            }
        }
        self.send(
            op,
            &r.path,
            Some(r.body.as_bytes().to_vec()),
            vec![],
            r.headers.clone(),
            Some(&r.idempotency_key),
            cancel,
        )
        .await
    }
    async fn call<T: DeserializeOwned>(
        &self,
        op: &str,
        path: String,
        body: Option<Value>,
        query: Vec<(String, String)>,
        headers: Vec<(String, String)>,
        key: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<T> {
        let bytes = body
            .as_ref()
            .map(|v| input(v, &operation(op)["input"]))
            .transpose()?;
        self.send(op, &path, bytes, query, headers, key, cancel)
            .await
    }
    async fn send<T: DeserializeOwned>(
        &self,
        op: &str,
        path: &str,
        body: Option<Vec<u8>>,
        query: Vec<(String, String)>,
        headers: Vec<(String, String)>,
        key: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<T> {
        let spec = operation(op);
        let method = spec["method"].as_str().unwrap();
        let mutation = method != "GET";
        if cancel.is_cancelled() {
            return Err(Error::transport("cancelled"));
        }
        let request = async {
            let mut url = reqwest::Url::parse(&format!("{}{}{}", self.origin, self.root, path))
                .map_err(|_| Error::input("invalid_request"))?;
            if !query.is_empty() {
                url.query_pairs_mut().extend_pairs(
                    query
                        .iter()
                        .map(|(key, value)| (key.as_str(), value.as_str())),
                );
            }
            let mut req = self
                .http
                .request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), url)
                .header("Accept", "application/json")
                .header("Accept-Encoding", "identity")
                .header("Cache-Control", "no-store");
            if let Some(secret) = &self.credential {
                req = req.bearer_auth(secret);
            }
            for (k, v) in headers {
                let mut value = reqwest::header::HeaderValue::from_str(&v)
                    .map_err(|_| Error::input("invalid_header"))?;
                value.set_sensitive(true);
                req = req.header(k, value);
            }
            if let Some(key) = key {
                let mut value = reqwest::header::HeaderValue::from_str(key)
                    .map_err(|_| Error::input("invalid_idempotency_key"))?;
                value.set_sensitive(true);
                req = req.header("Idempotency-Key", value);
            }
            if let Some(bytes) = &body {
                req = req
                    .header("Content-Type", "application/json")
                    .body(bytes.clone());
            }
            let mut res = req.send().await.map_err(|e| {
                Error::transport(if e.is_timeout() {
                    "timeout"
                } else {
                    "request_failed"
                })
            })?;
            let status = res.status().as_u16();
            let media = res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim();
            if (300..400).contains(&status)
                || !matches!(media, "application/json" | "application/problem+json")
                || (200..300).contains(&status) && media != "application/json"
                || res
                    .headers()
                    .get("content-encoding")
                    .is_some_and(|v| v != "identity")
            {
                return Err(Error::protocol("invalid_response", status));
            }
            if res.content_length().is_some_and(|n| n > self.max as u64) {
                return Err(Error::protocol("response_too_large", status));
            }
            let no_store = res
                .headers()
                .get("cache-control")
                .is_some_and(|v| v == "no-store");
            let mut bytes = Vec::new();
            while let Some(chunk) = res.chunk().await.map_err(|e| {
                Error::transport(if e.is_timeout() {
                    "timeout"
                } else {
                    "request_failed"
                })
            })? {
                if bytes.len() + chunk.len() > self.max {
                    return Err(Error::protocol("response_too_large", status));
                }
                bytes.extend_from_slice(&chunk);
            }
            let value = validation::strict_json(&bytes)
                .map_err(|_| Error::protocol("invalid_json", status))?;
            if !(200..300).contains(&status) {
                let code = value
                    .get("error")
                    .filter(|v| v.get("message").is_some_and(Value::is_string))
                    .and_then(|v| v.get("code"))
                    .and_then(Value::as_str)
                    .or_else(|| {
                        if value["type"] == "urn:availkit:problem" && value["status"] == status {
                            value["code"].as_str()
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| Error::protocol("invalid_error", status))?;
                return Err(Error {
                    kind: "api",
                    code: validation::safe_code(code).into(),
                    status: Some(status),
                    mutation_outcome_unknown: mutation
                        && (status >= 500 || validation::safe_code(code) == "api_error"),
                });
            }
            if status != spec["status"].as_u64().unwrap() as u16
                || !validate(&value, &spec["output"], 0)
                || (self.public
                    || path.starts_with("/usage")
                    || path.starts_with("/maintenance/")
                    || path.starts_with("/customers"))
                    && !no_store
            {
                return Err(Error::protocol("invalid_response", status));
            }
            relations::check(op, path, &query, body.as_deref(), &value, &self.root)
                .map_err(|_| Error::protocol("invalid_response", status))?;
            serde_json::from_value(value).map_err(|_| Error::protocol("invalid_response", status))
        };
        let result = tokio::select! { _=cancel.cancelled()=>Err(Error::transport("cancelled")), result=tokio::time::timeout(self.timeout,request)=>result.unwrap_or_else(|_|Err(Error::transport("timeout"))) };
        result.map_err(|mut error| {
            if mutation && (error.kind == "transport" || error.kind == "protocol") {
                error.mutation_outcome_unknown = true;
            }
            error
        })
    }
}

/// Original immutable bytes/key/authority. Debug is redacted. Export is explicit.
pub struct Intent<T> {
    record: IntentRecord,
    marker: PhantomData<T>,
}
impl<T> fmt::Debug for Intent<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AvailKit Intent (private)")
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IntentRecord {
    version: u8,
    operation: String,
    path: String,
    body: String,
    idempotency_key: String,
    scope: String,
    headers: Vec<(String, String)>,
    created_at_ms: u64,
}
impl<T> Intent<T> {
    /// Contains private capabilities/contact data. Never log or place in URLs.
    pub fn export_private(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.record).map_err(|_| Error::input("invalid_intent"))
    }
    pub fn import_private(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 131072 {
            return Err(Error::input("invalid_intent"));
        }
        let v = validation::strict_json(bytes).map_err(|_| Error::input("invalid_intent"))?;
        let record = serde_json::from_value(v).map_err(|_| Error::input("invalid_intent"))?;
        Ok(Self {
            record,
            marker: PhantomData,
        })
    }
    /// Exclusively create/fsync a0600 file in an existing owned0700 directory.
    #[cfg(unix)]
    pub fn save_private(&self, path: impl AsRef<Path>) -> Result<()> {
        private_files::create(path, &self.export_private()?)
    }
    #[cfg(unix)]
    pub fn load_private(path: impl AsRef<Path>) -> Result<Self> {
        Self::import_private(&private_files::read(path, 131072)?)
    }
}

#[derive(Clone, Debug)]
pub struct PageQuery {
    pub after: Option<String>,
    pub limit: u8,
    pub q: Option<String>,
}
impl Default for PageQuery {
    fn default() -> Self {
        Self {
            after: None,
            limit: 25,
            q: None,
        }
    }
}
impl PageQuery {
    fn pairs(&self) -> Result<Vec<(String, String)>> {
        if !(1..=100).contains(&self.limit) {
            return Err(Error::input("invalid_limit"));
        }
        let mut p = vec![("limit".into(), self.limit.to_string())];
        if let Some(a) = &self.after {
            identifier(a)?;
            p.push(("after".into(), a.clone()));
        }
        if let Some(q) = &self.q {
            if !validation::text(q, 160) {
                return Err(Error::input("invalid_query"));
            }
            p.push(("q".into(), q.clone()));
        }
        Ok(p)
    }
}
#[derive(Clone, Debug)]
pub struct AvailabilityQuery {
    pub offering_id: String,
    pub from: String,
    pub to: String,
}
impl AvailabilityQuery {
    fn pairs(&self) -> Result<Vec<(String, String)>> {
        identifier(&self.offering_id)?;
        let range = validation::instant(&self.to)? - validation::instant(&self.from)?;
        if range <= chrono::Duration::zero() || range > chrono::Duration::days(7) {
            return Err(Error::input("invalid_range"));
        }
        Ok(vec![
            ("offeringId".into(), self.offering_id.clone()),
            ("from".into(), self.from.clone()),
            ("to".into(), self.to.clone()),
        ])
    }
}
#[derive(Clone, Debug)]
pub struct EventQuery {
    pub after: u64,
    pub limit: u8,
}
impl Default for EventQuery {
    fn default() -> Self {
        Self {
            after: 0,
            limit: 100,
        }
    }
}
impl EventQuery {
    fn pairs(&self) -> Result<Vec<(String, String)>> {
        if self.after > 9007199254740991 || !(1..=100).contains(&self.limit) {
            return Err(Error::input("invalid_query"));
        }
        Ok(vec![
            ("after".into(), self.after.to_string()),
            ("limit".into(), self.limit.to_string()),
        ])
    }
}

/// Verify exact raw bytes, then transactionally deduplicate AvailKit-Delivery-ID.
pub fn verify_webhook(
    body: &[u8],
    signature: &str,
    secret: &str,
    now: u64,
    tolerance: u64,
) -> Result<bool> {
    use hmac::{Hmac, KeyInit, Mac};
    if tolerance > 3600 {
        return Err(Error::input("invalid_webhook_tolerance"));
    }
    if body.len() > 16384 || capability(secret).is_err() {
        return Ok(false);
    }
    let Some((stamp, digest)) = signature
        .strip_prefix("t=")
        .and_then(|s| s.split_once(",v1="))
    else {
        return Ok(false);
    };
    if stamp.is_empty()
        || stamp.len() > 20
        || stamp.len() > 1 && stamp.starts_with('0')
        || !stamp.bytes().all(|b| b.is_ascii_digit())
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Ok(false);
    }
    let Ok(t) = stamp.parse::<u64>() else {
        return Ok(false);
    };
    if now.abs_diff(t) > tolerance {
        return Ok(false);
    }
    let secret = URL_SAFE_NO_PAD
        .decode(secret)
        .map_err(|_| Error::input("invalid_secret"))?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&secret).map_err(|_| Error::input("invalid_secret"))?;
    mac.update(stamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let bytes: Vec<u8> = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&digest[i..i + 2], 16).unwrap())
        .collect();
    Ok(mac.verify_slice(&bytes).is_ok())
}
