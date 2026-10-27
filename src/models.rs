//! Wire types for the maintained stable contract. Validate before any network send.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceInput {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
    #[serde(rename = "capacity")]
    pub capacity: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
    #[serde(rename = "capacity")]
    pub capacity: u64,
    #[serde(rename = "archived")]
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePage {
    #[serde(rename = "items")]
    pub items: Vec<Resource>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoursWindowsItem {
    #[serde(rename = "weekday")]
    pub weekday: u64,
    #[serde(rename = "start")]
    pub start: String,
    #[serde(rename = "end")]
    pub end: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hours {
    #[serde(rename = "windows")]
    pub windows: Vec<HoursWindowsItem>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferingInput {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offering {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: u64,
    #[serde(rename = "archived")]
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferingPage {
    #[serde(rename = "items")]
    pub items: Vec<Offering>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilitySlotsItem {
    #[serde(rename = "start")]
    pub start: String,
    #[serde(rename = "end")]
    pub end: String,
    #[serde(rename = "remainingCapacity")]
    pub remaining_capacity: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    #[serde(rename = "slots")]
    pub slots: Vec<AvailabilitySlotsItem>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookingInput {
    #[serde(rename = "offeringId")]
    pub offering_id: String,
    #[serde(rename = "start")]
    pub start: String,
    #[serde(
        rename = "customerId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub customer_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookingStatus {
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "cancelled")]
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Booking {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "offeringId")]
    pub offering_id: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "start")]
    pub start: String,
    #[serde(rename = "end")]
    pub end: String,
    #[serde(rename = "status")]
    pub status: BookingStatus,
    #[serde(
        rename = "customerId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub customer_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookingPage {
    #[serde(rename = "items")]
    pub items: Vec<Booking>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventsEventsItemType {
    #[serde(rename = "booking.created")]
    BookingCreated,
    #[serde(rename = "booking.cancelled")]
    BookingCancelled,
    #[serde(rename = "booking.rescheduled")]
    BookingRescheduled,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsEventsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "sequence")]
    pub sequence: u64,
    #[serde(rename = "type")]
    pub kind: EventsEventsItemType,
    #[serde(rename = "bookingId")]
    pub booking_id: String,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Events {
    #[serde(rename = "events")]
    pub events: Vec<EventsEventsItem>,
    #[serde(rename = "nextAfter")]
    pub next_after: u64,
    #[serde(rename = "retentionFloor")]
    pub retention_floor: u64,
    #[serde(rename = "highWater")]
    pub high_water: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookCreate {
    #[serde(rename = "url")]
    pub url: String,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookCreated {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "signingSecret")]
    pub signing_secret: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookPage {
    #[serde(rename = "items")]
    pub items: Vec<Webhook>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookDisable {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookDeliveryPage {
    #[serde(rename = "items")]
    pub items: Vec<WebhookDelivery>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookDeliveryState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "suppressed")]
    Suppressed,
    #[serde(rename = "leased")]
    Leased,
    #[serde(rename = "sending")]
    Sending,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "dead")]
    Dead,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookDelivery {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "subscriptionId")]
    pub subscription_id: String,
    #[serde(rename = "eventId")]
    pub event_id: String,
    #[serde(rename = "state")]
    pub state: WebhookDeliveryState,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "attempts")]
    pub attempts: u64,
    #[serde(rename = "maxAttempts")]
    pub max_attempts: u64,
    #[serde(rename = "nextAttemptAt")]
    pub next_attempt_at: Option<String>,
    #[serde(rename = "leaseExpiresAt")]
    pub lease_expires_at: Option<String>,
    #[serde(rename = "lastAttemptAt")]
    pub last_attempt_at: Option<String>,
    #[serde(rename = "lastOutcome")]
    pub last_outcome: Option<String>,
    #[serde(rename = "lastHttpStatus")]
    pub last_http_status: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    #[serde(rename = "asOf")]
    pub as_of: String,
    #[serde(rename = "period")]
    pub period: UsagePeriod,
    #[serde(rename = "counters")]
    pub counters: UsageCounters,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsagePeriod {
    #[serde(rename = "start")]
    pub start: String,
    #[serde(rename = "end")]
    pub end: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageCounters {
    #[serde(rename = "monthlyBookingCreates")]
    pub monthly_booking_creates: UsageCounter,
    #[serde(rename = "catalogObjects")]
    pub catalog_objects: UsageCounter,
    #[serde(rename = "bookingRecords")]
    pub booking_records: UsageCounter,
    #[serde(rename = "committedEvents")]
    pub committed_events: UsageCounter,
    #[serde(rename = "webhookSubscriptions")]
    pub webhook_subscriptions: UsageCounter,
    #[serde(rename = "webhookDeliveries")]
    pub webhook_deliveries: UsageCounter,
    #[serde(rename = "liveReplays")]
    pub live_replays: UsageCounter,
    #[serde(rename = "customerRecords")]
    pub customer_records: UsageCounter,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageCounter {
    #[serde(rename = "used")]
    pub used: u64,
    #[serde(rename = "limit")]
    pub limit: u64,
    #[serde(rename = "remaining")]
    pub remaining: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RescheduleInput {
    #[serde(rename = "expectedStart")]
    pub expected_start: String,
    #[serde(rename = "start")]
    pub start: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryPruneInputKind {
    #[serde(rename = "deliveries")]
    Deliveries,
    #[serde(rename = "events")]
    Events,
    #[serde(rename = "bookings")]
    Bookings,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPruneInput {
    #[serde(rename = "kind")]
    pub kind: HistoryPruneInputKind,
    #[serde(rename = "limit")]
    pub limit: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPruneResult {
    #[serde(rename = "asOf")]
    pub as_of: String,
    #[serde(rename = "cutoff")]
    pub cutoff: String,
    #[serde(rename = "deleted")]
    pub deleted: u64,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomerInput {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "email", default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "phone", default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Customer {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "email")]
    pub email: Option<String>,
    #[serde(rename = "phone")]
    pub phone: Option<String>,
    #[serde(rename = "archived")]
    pub archived: bool,
    #[serde(rename = "erased", default, skip_serializing_if = "Option::is_none")]
    pub erased: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomerPage {
    #[serde(rename = "items")]
    pub items: Vec<Customer>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomerErased {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "erased")]
    pub erased: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceInput {
    #[serde(rename = "limit")]
    pub limit: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceResult {
    #[serde(rename = "asOf")]
    pub as_of: String,
    #[serde(rename = "deleted")]
    pub deleted: u64,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicPageEnvironment {
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "live")]
    Live,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicPage {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "environment")]
    pub environment: PublicPageEnvironment,
    #[serde(
        rename = "emailNotifications",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub email_notifications: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicOfferingPage {
    #[serde(rename = "items")]
    pub items: Vec<PublicOffering>,
    #[serde(rename = "nextAfter")]
    pub next_after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicOffering {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: u64,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicBookingInput {
    #[serde(rename = "offeringId")]
    pub offering_id: String,
    #[serde(rename = "start")]
    pub start: String,
    #[serde(rename = "contact")]
    pub contact: CustomerInput,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicBookingStatus {
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "cancelled")]
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicBooking {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "start")]
    pub start: String,
    #[serde(rename = "end")]
    pub end: String,
    #[serde(rename = "status")]
    pub status: PublicBookingStatus,
    #[serde(rename = "offering")]
    pub offering: PublicOffering,
    #[serde(rename = "page")]
    pub page: PublicPage,
    #[serde(rename = "pageEnabled")]
    pub page_enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookingPageSettings {
    #[serde(rename = "pageId")]
    pub page_id: Option<String>,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "needsRepublish")]
    pub needs_republish: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookingPageInput {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

impl std::fmt::Debug for WebhookCreated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AvailKit WebhookCreated (private)")
    }
}

impl std::fmt::Debug for Customer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AvailKit Customer (private)")
    }
}

impl std::fmt::Debug for CustomerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AvailKit CustomerInput (private)")
    }
}
