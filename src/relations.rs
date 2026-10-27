use crate::{Error, Result, validation::instant};
use chrono::{Datelike, TimeZone};
use serde_json::Value;
pub(crate) fn check(
    op: &str,
    path: &str,
    q: &[(String, String)],
    body: Option<&[u8]>,
    v: &Value,
    root: &str,
) -> Result<()> {
    let bad = || Error::input("invalid_response");
    let get = |key: &str, default: &str| {
        q.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.to_string())
            .unwrap_or(default.into())
    };
    if matches!(op, "getHours" | "setHours") {
        crate::validation::input(v, &serde_json::json!({"$ref":"#/components/schemas/Hours"}))?;
    }
    let b: Value = body
        .map(|b| serde_json::from_slice(b).unwrap_or(Value::Null))
        .unwrap_or(Value::Null);
    if let Some(items) = v["items"].as_array() {
        let limit = get("limit", "25").parse::<usize>().map_err(|_| bad())?;
        let mut after = get("after", "");
        if items.len() > limit {
            return Err(bad());
        }
        for item in items {
            let id = item["id"].as_str().ok_or_else(bad)?;
            crate::validation::identifier(id)?;
            if id <= after.as_str() {
                return Err(bad());
            }
            after = id.into();
            if op == "listWebhookDeliveries"
                && item["subscriptionId"] != path.split('/').nth(2).unwrap_or("")
            {
                return Err(bad());
            }
        }
        if !v["nextAfter"].is_null() && (items.len() != limit || v["nextAfter"] != after) {
            return Err(bad());
        }
    }
    if matches!(
        op,
        "getResource"
            | "updateResource"
            | "archiveResource"
            | "getOffering"
            | "updateOffering"
            | "archiveOffering"
            | "getBooking"
            | "cancel"
            | "rescheduleBooking"
            | "getCustomer"
            | "replaceCustomer"
            | "archiveCustomer"
            | "eraseCustomer"
            | "disableWebhook"
            | "getPublicBooking"
            | "cancelPublicBooking"
            | "reschedulePublicBooking"
    ) && v["id"] != path.split('/').nth(2).unwrap_or("")
    {
        return Err(bad());
    }
    let ts = |o: &Value, k: &str| instant(o[k].as_str().ok_or_else(bad)?);
    if matches!(
        op,
        "book"
            | "getBooking"
            | "cancel"
            | "rescheduleBooking"
            | "getPublicBooking"
            | "bookPublic"
            | "cancelPublicBooking"
            | "reschedulePublicBooking"
    ) && ts(v, "end")? <= ts(v, "start")?
    {
        return Err(bad());
    }
    if matches!(op, "cancel" | "cancelPublicBooking") && v["status"] != "cancelled" {
        return Err(bad());
    }
    if matches!(
        op,
        "book" | "bookPublic" | "rescheduleBooking" | "reschedulePublicBooking"
    ) && (v["status"] != "confirmed" || ts(v, "start")? != ts(&b, "start")?)
    {
        return Err(bad());
    }
    if op == "book"
        && (v["offeringId"] != b["offeringId"] || v.get("customerId") != b.get("customerId"))
    {
        return Err(bad());
    }
    if op == "bookPublic" && v["offering"]["id"] != b["offeringId"] {
        return Err(bad());
    }
    if matches!(
        op,
        "archiveResource" | "archiveOffering" | "archiveCustomer"
    ) && v["archived"] != true
        || op == "disableWebhook" && v["enabled"] != false
    {
        return Err(bad());
    }
    if root.contains("/public/") {
        let page = root.rsplit('/').next().unwrap();
        if op == "getPublicPage" && v["id"] != page
            || v.get("page").is_some() && v["page"]["id"] != page
        {
            return Err(bad());
        }
    }
    if matches!(op, "availability" | "publicAvailability") {
        let lo = instant(&get("from", ""))?;
        let hi = instant(&get("to", ""))?;
        let mut prev = None;
        for slot in v["slots"].as_array().ok_or_else(bad)? {
            let start = ts(slot, "start")?;
            let end = ts(slot, "end")?;
            if start < lo
                || end > hi
                || start >= end
                || prev.is_some_and(|p| start <= p)
                || slot["remainingCapacity"].as_u64().unwrap_or(0) == 0
            {
                return Err(bad());
            }
            prev = Some(start);
        }
    }
    if matches!(op, "pruneHistory" | "pruneExpiredReplays") {
        let deleted = v["deleted"].as_u64().ok_or_else(bad)?;
        let limit = b["limit"].as_u64().ok_or_else(bad)?;
        if deleted > limit
            || v["hasMore"] == true && deleted != limit
            || op == "pruneHistory"
                && ts(v, "asOf")? - ts(v, "cutoff")? != chrono::Duration::days(30)
        {
            return Err(bad());
        }
    }
    if op == "events" {
        let a = get("after", "0").parse::<u64>().map_err(|_| bad())?;
        let floor = v["retentionFloor"].as_u64().ok_or_else(bad)?;
        let high = v["highWater"].as_u64().ok_or_else(bad)?;
        let mut prev = a.max(floor);
        let events = v["events"].as_array().ok_or_else(bad)?;
        if floor > high || events.len() > get("limit", "100").parse::<usize>().map_err(|_| bad())? {
            return Err(bad());
        }
        for e in events {
            let seq = e["sequence"].as_u64().ok_or_else(bad)?;
            if seq <= prev || seq > high {
                return Err(bad());
            }
            prev = seq;
        }
        if v["nextAfter"] != prev {
            return Err(bad());
        }
    }
    if op == "getUsage" {
        let as_of = ts(v, "asOf")?;
        let start = chrono::Utc
            .with_ymd_and_hms(as_of.year(), as_of.month(), 1, 0, 0, 0)
            .single()
            .ok_or_else(bad)?;
        let (y, m) = if as_of.month() == 12 {
            (as_of.year() + 1, 1)
        } else {
            (as_of.year(), as_of.month() + 1)
        };
        let end = chrono::Utc
            .with_ymd_and_hms(y, m, 1, 0, 0, 0)
            .single()
            .ok_or_else(bad)?;
        if ts(&v["period"], "start")? != start || ts(&v["period"], "end")? != end {
            return Err(bad());
        }
        for c in v["counters"].as_object().ok_or_else(bad)?.values() {
            let used = c["used"].as_u64().ok_or_else(bad)?;
            let limit = c["limit"].as_u64().ok_or_else(bad)?;
            if c["remaining"] != limit.saturating_sub(used) {
                return Err(bad());
            }
        }
    }
    Ok(())
}
