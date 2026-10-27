use crate::{Error, Result};
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};
pub(crate) fn identifier(s: &str) -> Result<()> {
    if s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(Error::input("invalid_id"))
    }
}
pub(crate) fn capability(s: &str) -> Result<()> {
    if s.len() == 43
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && b"AEIMQUYcgkosw048".contains(&s.as_bytes()[42])
    {
        Ok(())
    } else {
        Err(Error::input("invalid_capability"))
    }
}
pub(crate) fn text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}
pub(crate) fn instant(s: &str) -> Result<chrono::DateTime<chrono::FixedOffset>> {
    if !s.ends_with('Z')
        || s.len() < 20
        || s.as_bytes().get(10) != Some(&b'T')
        || s.split('.').nth(1).is_some_and(|v| v.len() > 10)
    {
        return Err(Error::input("invalid_timestamp"));
    }
    chrono::DateTime::parse_from_rfc3339(s).map_err(|_| Error::input("invalid_timestamp"))
}
fn schemas() -> &'static Value {
    static S: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    S.get_or_init(|| serde_json::from_str(include_str!("schemas.json")).expect("bundled schemas"))
}
fn pattern(s: &str, p: &str) -> bool {
    match p {
        "^[0-9a-f]{32}$" => identifier(s).is_ok(),
        "^[A-Za-z0-9_-]{43}$" => capability(s).is_ok(),
        "Z$" => s.ends_with('Z'),
        "^[0-2][0-9]:[0-5][0-9]$" => {
            let b = s.as_bytes();
            b.len() == 5
                && (b'0'..=b'2').contains(&b[0])
                && b[1].is_ascii_digit()
                && b[2] == b':'
                && (b'0'..=b'5').contains(&b[3])
                && b[4].is_ascii_digit()
        }
        _ => false,
    }
}
pub(crate) fn validate(v: &Value, s: &Value, depth: u8) -> bool {
    if depth > 20 {
        return false;
    }
    if let Some(r) = s["$ref"].as_str() {
        return validate(v, &schemas()[r.rsplit('/').next().unwrap()], depth + 1);
    }
    for (k, one) in [("allOf", false), ("anyOf", false), ("oneOf", true)] {
        if let Some(a) = s[k].as_array() {
            let n = a.iter().filter(|s| validate(v, s, depth + 1)).count();
            if k == "allOf" && n != a.len() || k != "allOf" && n == 0 || one && n != 1 {
                return false;
            }
        }
    }
    if s.get("const").is_some_and(|c| c != v)
        || s["enum"].as_array().is_some_and(|a| !a.contains(v))
    {
        return false;
    }
    if let Some(t) = s.get("type") {
        let check = |t: &str| match t {
            "null" => v.is_null(),
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "integer" => v.as_i64().is_some() || v.as_u64().is_some(),
            "boolean" => v.is_boolean(),
            _ => false,
        };
        let ok = if let Some(t) = t.as_str() {
            check(t)
        } else {
            t.as_array()
                .is_some_and(|a| a.iter().any(|t| check(t.as_str().unwrap_or(""))))
        };
        if !ok {
            return false;
        }
    }
    if let Some(o) = v.as_object() {
        let props = s.get("properties").and_then(Value::as_object);
        if s["required"]
            .as_array()
            .is_some_and(|a| a.iter().any(|k| !o.contains_key(k.as_str().unwrap())))
        {
            return false;
        }
        for (k, v) in o {
            if matches!(
                k.as_str(),
                "id" | "resourceId"
                    | "offeringId"
                    | "customerId"
                    | "pageId"
                    | "subscriptionId"
                    | "eventId"
                    | "bookingId"
            ) && !v.is_null()
                && !v.as_str().is_some_and(|id| identifier(id).is_ok())
            {
                return false;
            }
            if k == "signingSecret" && !v.as_str().is_some_and(|secret| capability(secret).is_ok())
            {
                return false;
            }
            let p = props.and_then(|p| p.get(k));
            if p.is_none() && s["additionalProperties"] == false {
                return false;
            }
            if p.is_some_and(|p| !validate(v, p, depth + 1)) {
                return false;
            }
        }
    }
    if let Some(a) = v.as_array() {
        if a.len() < s["minItems"].as_u64().unwrap_or(0) as usize
            || a.len() > s["maxItems"].as_u64().unwrap_or(10000) as usize
            || a.iter().any(|v| !validate(v, &s["items"], depth + 1))
        {
            return false;
        }
    }
    if let Some(t) = v.as_str() {
        if t.chars().count() < s["minLength"].as_u64().unwrap_or(0) as usize
            || t.chars().count() > s["maxLength"].as_u64().unwrap_or(8192) as usize
            || s["pattern"].as_str().is_some_and(|p| !pattern(t, p))
            || s["format"] == "date-time" && instant(t).is_err()
        {
            return false;
        }
    }
    if let Some(n) = v.as_f64() {
        if n < s["minimum"].as_f64().unwrap_or(-9007199254740991.)
            || n > s["maximum"].as_f64().unwrap_or(9007199254740991.)
        {
            return false;
        }
    }
    true
}
pub(crate) fn input(v: &Value, s: &Value) -> Result<Vec<u8>> {
    if !validate(v, s, 0) {
        return Err(Error::input("invalid_input"));
    }
    let name = s["$ref"]
        .as_str()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    let string = |k: &str| v[k].as_str().unwrap_or("");
    if matches!(name, "ResourceInput" | "OfferingInput") && !text(string("name"), 160)
        || name == "BookingPageInput" && !text(string("title"), 160)
    {
        return Err(Error::input("invalid_input"));
    }
    if name == "ResourceInput" && !text(string("timeZone"), 100) {
        return Err(Error::input("invalid_time_zone"));
    }
    if name == "OfferingInput" {
        identifier(string("resourceId"))?;
    }
    if matches!(name, "BookingInput" | "PublicBookingInput") {
        identifier(string("offeringId"))?;
    }
    if name == "PublicBookingInput" {
        input(
            &v["contact"],
            &serde_json::json!({"$ref":"#/components/schemas/CustomerInput"}),
        )?;
    }
    if name == "CustomerInput" {
        let email = v["email"].as_str();
        if !text(string("displayName"), 160)
            || email.is_some_and(|e| {
                !text(e, 254)
                    || e.split('@').count() != 2
                    || e.starts_with('@')
                    || e.ends_with('@')
                    || e.chars().any(char::is_whitespace)
            })
            || v["phone"].as_str().is_some_and(|p| !text(p, 32))
        {
            return Err(Error::input("invalid_customer"));
        }
    }
    if name == "Hours" {
        let a = v["windows"].as_array().unwrap();
        for (i, w) in a.iter().enumerate() {
            let start = w["start"].as_str().unwrap();
            let end = w["end"].as_str().unwrap();
            if start >= "24:00"
                || end > "24:00"
                || start >= end
                || a[..i].iter().any(|p| {
                    p["weekday"] == w["weekday"]
                        && p["start"].as_str().unwrap() < end
                        && start < p["end"].as_str().unwrap()
                })
            {
                return Err(Error::input("invalid_hours"));
            }
        }
    }
    if name == "WebhookCreate" {
        let u =
            reqwest::Url::parse(string("url")).map_err(|_| Error::input("invalid_webhook_url"))?;
        if u.scheme() != "https"
            || u.host_str().is_none()
            || u.host_str()
                .is_some_and(|h| h.parse::<std::net::IpAddr>().is_ok())
            || u.port_or_known_default() != Some(443)
            || !u.username().is_empty()
            || u.password().is_some()
            || u.query().is_some()
            || u.fragment().is_some()
        {
            return Err(Error::input("invalid_webhook_url"));
        }
    }
    let b = serde_json::to_vec(v).map_err(|_| Error::input("invalid_json"))?;
    if b.len() > 16384 {
        return Err(Error::input("body_too_large"));
    }
    Ok(b)
}
pub(crate) fn safe_code(s: &str) -> &str {
    const C: &str = "booking_page_paused booking_access_in_use contact_changed validation_failed synthetic_email_required unauthorized forbidden scope_denied not_found invalid_input invalid_query invalid_cursor invalid_limit quota_exceeded unavailable credential_invalid capability_denied customer_archived idempotency_conflict capacity_conflict capacity_exceeded version_conflict catalog_in_use catalog_archived event_cursor_expired invalid_id invalid_range identity_unavailable recovery_unavailable csrf_invalid recent_auth_required secret_already_revealed idempotency_mismatch membership_changed membership_denied last_credential identity_limit identity_rate_limited membership_conflict request_key_invalid";
    if C.split(' ').any(|c| c == s) {
        s
    } else {
        "api_error"
    }
}
// serde_json normally accepts duplicate object keys. Reject them at every depth.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("unique JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(n.into()))
                    .ok_or_else(|| E::custom("number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut v = Vec::new();
                while let Some(Strict(x)) = a.next_element()? {
                    v.push(x);
                }
                Ok(Strict(v.into()))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut m = BTreeMap::new();
                while let Some((k, Strict(v))) = a.next_entry::<String, Strict>()? {
                    if m.insert(k, v).is_some() {
                        return Err(serde::de::Error::custom("duplicate"));
                    }
                }
                Ok(Strict(
                    serde_json::to_value(m).map_err(serde::de::Error::custom)?,
                ))
            }
        }
        d.deserialize_any(V)
    }
}
pub(crate) fn strict_json(bytes: &[u8]) -> std::result::Result<Value, serde_json::Error> {
    serde_json::from_slice::<Strict>(bytes).map(|s| s.0)
}
