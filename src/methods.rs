// Generated stable operation methods; regenerate with python3 generate.py.
use crate::models::*;
use crate::*;
impl Client {
    pub async fn create_resource(
        &self,
        cancel: &CancellationToken,
        body: &ResourceInput,
    ) -> Result<Resource> {
        let path = format!("/resources");
        self.inner
            .call(
                "createResource",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn list_resources(
        &self,
        cancel: &CancellationToken,
        page: &PageQuery,
    ) -> Result<ResourcePage> {
        let path = format!("/resources");
        self.inner
            .call(
                "listResources",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn replace_hours(
        &self,
        cancel: &CancellationToken,
        resource_id: &str,
        body: &Hours,
    ) -> Result<Hours> {
        identifier(resource_id)?;
        let path = format!("/resources/{resource_id}/hours");
        self.inner
            .call(
                "setHours",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn get_hours(&self, cancel: &CancellationToken, resource_id: &str) -> Result<Hours> {
        identifier(resource_id)?;
        let path = format!("/resources/{resource_id}/hours");
        self.inner
            .call("getHours", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn create_offering(
        &self,
        cancel: &CancellationToken,
        body: &OfferingInput,
    ) -> Result<Offering> {
        let path = format!("/offerings");
        self.inner
            .call(
                "createOffering",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn list_offerings(
        &self,
        cancel: &CancellationToken,
        page: &PageQuery,
    ) -> Result<OfferingPage> {
        let path = format!("/offerings");
        self.inner
            .call(
                "listOfferings",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn availability(
        &self,
        cancel: &CancellationToken,
        range: &AvailabilityQuery,
    ) -> Result<Availability> {
        let path = format!("/availability");
        self.inner
            .call(
                "availability",
                path,
                None,
                range.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub fn prepare_book(
        &self,
        body: &BookingInput,
        idempotency_key: &str,
    ) -> Result<Intent<Booking>> {
        let path = format!("/bookings");
        self.inner.prepare(
            "book",
            path,
            Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?).unwrap(),
            idempotency_key,
            vec![],
        )
    }
    pub async fn book(
        &self,
        cancel: &CancellationToken,
        body: &BookingInput,
        idempotency_key: &str,
    ) -> Result<Booking> {
        self.execute(&self.prepare_book(body, idempotency_key)?, cancel)
            .await
    }
    pub async fn list_bookings(
        &self,
        cancel: &CancellationToken,
        page: &PageQuery,
    ) -> Result<BookingPage> {
        let path = format!("/bookings");
        self.inner
            .call(
                "listBookings",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub fn prepare_cancel(
        &self,
        booking_id: &str,
        idempotency_key: &str,
    ) -> Result<Intent<Booking>> {
        identifier(booking_id)?;
        let path = format!("/bookings/{booking_id}/cancel");
        self.inner.prepare(
            "cancel",
            path,
            Some(serde_json::json!({})).unwrap(),
            idempotency_key,
            vec![],
        )
    }
    pub async fn cancel(
        &self,
        cancel: &CancellationToken,
        booking_id: &str,
        idempotency_key: &str,
    ) -> Result<Booking> {
        self.execute(&self.prepare_cancel(booking_id, idempotency_key)?, cancel)
            .await
    }
    pub async fn events(&self, cancel: &CancellationToken, page: &EventQuery) -> Result<Events> {
        let path = format!("/events");
        self.inner
            .call("events", path, None, page.pairs()?, vec![], None, cancel)
            .await
    }
    pub async fn get_resource(
        &self,
        cancel: &CancellationToken,
        resource_id: &str,
    ) -> Result<Resource> {
        identifier(resource_id)?;
        let path = format!("/resources/{resource_id}");
        self.inner
            .call("getResource", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn replace_resource(
        &self,
        cancel: &CancellationToken,
        resource_id: &str,
        body: &ResourceInput,
    ) -> Result<Resource> {
        identifier(resource_id)?;
        let path = format!("/resources/{resource_id}");
        self.inner
            .call(
                "updateResource",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn get_offering(
        &self,
        cancel: &CancellationToken,
        offering_id: &str,
    ) -> Result<Offering> {
        identifier(offering_id)?;
        let path = format!("/offerings/{offering_id}");
        self.inner
            .call("getOffering", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn replace_offering(
        &self,
        cancel: &CancellationToken,
        offering_id: &str,
        body: &OfferingInput,
    ) -> Result<Offering> {
        identifier(offering_id)?;
        let path = format!("/offerings/{offering_id}");
        self.inner
            .call(
                "updateOffering",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn get_booking(
        &self,
        cancel: &CancellationToken,
        booking_id: &str,
    ) -> Result<Booking> {
        identifier(booking_id)?;
        let path = format!("/bookings/{booking_id}");
        self.inner
            .call("getBooking", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn create_webhook(
        &self,
        cancel: &CancellationToken,
        body: &WebhookCreate,
    ) -> Result<WebhookCreated> {
        let path = format!("/webhooks");
        self.inner
            .call(
                "createWebhook",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn list_webhooks(
        &self,
        cancel: &CancellationToken,
        page: &PageQuery,
    ) -> Result<WebhookPage> {
        let path = format!("/webhooks");
        self.inner
            .call(
                "listWebhooks",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn disable_webhook(
        &self,
        cancel: &CancellationToken,
        subscription_id: &str,
    ) -> Result<Webhook> {
        identifier(subscription_id)?;
        let path = format!("/webhooks/{subscription_id}/disable");
        self.inner
            .call(
                "disableWebhook",
                path,
                Some(serde_json::json!({})),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn list_webhook_deliveries(
        &self,
        cancel: &CancellationToken,
        subscription_id: &str,
        page: &PageQuery,
    ) -> Result<WebhookDeliveryPage> {
        identifier(subscription_id)?;
        let path = format!("/webhooks/{subscription_id}/deliveries");
        self.inner
            .call(
                "listWebhookDeliveries",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn get_usage(&self, cancel: &CancellationToken) -> Result<Usage> {
        let path = format!("/usage");
        self.inner
            .call("getUsage", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub fn prepare_reschedule_booking(
        &self,
        booking_id: &str,
        body: &RescheduleInput,
        idempotency_key: &str,
    ) -> Result<Intent<Booking>> {
        identifier(booking_id)?;
        let path = format!("/bookings/{booking_id}/reschedule");
        self.inner.prepare(
            "rescheduleBooking",
            path,
            Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?).unwrap(),
            idempotency_key,
            vec![],
        )
    }
    pub async fn reschedule_booking(
        &self,
        cancel: &CancellationToken,
        booking_id: &str,
        body: &RescheduleInput,
        idempotency_key: &str,
    ) -> Result<Booking> {
        self.execute(
            &self.prepare_reschedule_booking(booking_id, body, idempotency_key)?,
            cancel,
        )
        .await
    }
    pub async fn archive_resource(
        &self,
        cancel: &CancellationToken,
        resource_id: &str,
    ) -> Result<Resource> {
        identifier(resource_id)?;
        let path = format!("/resources/{resource_id}/archive");
        self.inner
            .call(
                "archiveResource",
                path,
                Some(serde_json::json!({})),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn archive_offering(
        &self,
        cancel: &CancellationToken,
        offering_id: &str,
    ) -> Result<Offering> {
        identifier(offering_id)?;
        let path = format!("/offerings/{offering_id}/archive");
        self.inner
            .call(
                "archiveOffering",
                path,
                Some(serde_json::json!({})),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn prune_history(
        &self,
        cancel: &CancellationToken,
        body: &HistoryPruneInput,
    ) -> Result<HistoryPruneResult> {
        let path = format!("/maintenance/history");
        self.inner
            .call(
                "pruneHistory",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn create_customer(
        &self,
        cancel: &CancellationToken,
        body: &CustomerInput,
    ) -> Result<Customer> {
        let path = format!("/customers");
        self.inner
            .call(
                "createCustomer",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn list_customers(
        &self,
        cancel: &CancellationToken,
        page: &PageQuery,
    ) -> Result<CustomerPage> {
        let path = format!("/customers");
        self.inner
            .call(
                "listCustomers",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn get_customer(
        &self,
        cancel: &CancellationToken,
        customer_id: &str,
    ) -> Result<Customer> {
        identifier(customer_id)?;
        let path = format!("/customers/{customer_id}");
        self.inner
            .call("getCustomer", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn replace_customer(
        &self,
        cancel: &CancellationToken,
        customer_id: &str,
        body: &CustomerInput,
    ) -> Result<Customer> {
        identifier(customer_id)?;
        let path = format!("/customers/{customer_id}");
        self.inner
            .call(
                "replaceCustomer",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn archive_customer(
        &self,
        cancel: &CancellationToken,
        customer_id: &str,
    ) -> Result<Customer> {
        identifier(customer_id)?;
        let path = format!("/customers/{customer_id}/archive");
        self.inner
            .call(
                "archiveCustomer",
                path,
                Some(serde_json::json!({})),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn erase_customer(
        &self,
        cancel: &CancellationToken,
        customer_id: &str,
    ) -> Result<CustomerErased> {
        identifier(customer_id)?;
        let path = format!("/customers/{customer_id}/erase");
        self.inner
            .call(
                "eraseCustomer",
                path,
                Some(serde_json::json!({})),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn prune_expired_replays(
        &self,
        cancel: &CancellationToken,
        body: &MaintenanceInput,
    ) -> Result<MaintenanceResult> {
        let path = format!("/maintenance/expired-replays");
        self.inner
            .call(
                "pruneExpiredReplays",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn get_booking_page(
        &self,
        cancel: &CancellationToken,
    ) -> Result<BookingPageSettings> {
        let path = format!("/booking-page");
        self.inner
            .call("getBookingPage", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn replace_booking_page(
        &self,
        cancel: &CancellationToken,
        body: &BookingPageInput,
    ) -> Result<BookingPageSettings> {
        let path = format!("/booking-page");
        self.inner
            .call(
                "replaceBookingPage",
                path,
                Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?),
                vec![],
                vec![],
                None,
                cancel,
            )
            .await
    }
}
impl PublicClient {
    pub async fn get_page(&self, cancel: &CancellationToken) -> Result<PublicPage> {
        let path = format!("");
        self.inner
            .call("getPublicPage", path, None, vec![], vec![], None, cancel)
            .await
    }
    pub async fn list_offerings(
        &self,
        cancel: &CancellationToken,
        page: &PageQuery,
    ) -> Result<PublicOfferingPage> {
        let path = format!("/offerings");
        self.inner
            .call(
                "listPublicOfferings",
                path,
                None,
                page.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub async fn availability(
        &self,
        cancel: &CancellationToken,
        range: &AvailabilityQuery,
    ) -> Result<Availability> {
        let path = format!("/availability");
        self.inner
            .call(
                "publicAvailability",
                path,
                None,
                range.pairs()?,
                vec![],
                None,
                cancel,
            )
            .await
    }
    pub fn prepare_book(
        &self,
        body: &PublicBookingInput,
        booking_token: &str,
        contact_token: &str,
        idempotency_key: &str,
    ) -> Result<Intent<PublicBooking>> {
        capability(booking_token)?;
        capability(contact_token)?;
        let path = format!("/bookings");
        self.inner.prepare(
            "bookPublic",
            path,
            Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?).unwrap(),
            idempotency_key,
            vec![
                ("Authorization".into(), format!("Booking {booking_token}")),
                ("X-AvailKit-Contact-Token".into(), contact_token.into()),
            ],
        )
    }
    pub async fn book(
        &self,
        cancel: &CancellationToken,
        body: &PublicBookingInput,
        booking_token: &str,
        contact_token: &str,
        idempotency_key: &str,
    ) -> Result<PublicBooking> {
        self.execute(
            &self.prepare_book(body, booking_token, contact_token, idempotency_key)?,
            cancel,
        )
        .await
    }
    pub async fn get_booking(
        &self,
        cancel: &CancellationToken,
        booking_id: &str,
        booking_token: &str,
    ) -> Result<PublicBooking> {
        identifier(booking_id)?;
        capability(booking_token)?;
        let path = format!("/bookings/{booking_id}");
        self.inner
            .call(
                "getPublicBooking",
                path,
                None,
                vec![],
                vec![("Authorization".into(), format!("Booking {booking_token}"))],
                None,
                cancel,
            )
            .await
    }
    pub fn prepare_cancel(
        &self,
        booking_id: &str,
        booking_token: &str,
        idempotency_key: &str,
    ) -> Result<Intent<PublicBooking>> {
        identifier(booking_id)?;
        capability(booking_token)?;
        let path = format!("/bookings/{booking_id}/cancel");
        self.inner.prepare(
            "cancelPublicBooking",
            path,
            Some(serde_json::json!({})).unwrap(),
            idempotency_key,
            vec![("Authorization".into(), format!("Booking {booking_token}"))],
        )
    }
    pub async fn cancel(
        &self,
        cancel: &CancellationToken,
        booking_id: &str,
        booking_token: &str,
        idempotency_key: &str,
    ) -> Result<PublicBooking> {
        self.execute(
            &self.prepare_cancel(booking_id, booking_token, idempotency_key)?,
            cancel,
        )
        .await
    }
    pub fn prepare_reschedule(
        &self,
        booking_id: &str,
        body: &RescheduleInput,
        booking_token: &str,
        idempotency_key: &str,
    ) -> Result<Intent<PublicBooking>> {
        identifier(booking_id)?;
        capability(booking_token)?;
        let path = format!("/bookings/{booking_id}/reschedule");
        self.inner.prepare(
            "reschedulePublicBooking",
            path,
            Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?).unwrap(),
            idempotency_key,
            vec![("Authorization".into(), format!("Booking {booking_token}"))],
        )
    }
    pub async fn reschedule(
        &self,
        cancel: &CancellationToken,
        booking_id: &str,
        body: &RescheduleInput,
        booking_token: &str,
        idempotency_key: &str,
    ) -> Result<PublicBooking> {
        self.execute(
            &self.prepare_reschedule(booking_id, body, booking_token, idempotency_key)?,
            cancel,
        )
        .await
    }
}
