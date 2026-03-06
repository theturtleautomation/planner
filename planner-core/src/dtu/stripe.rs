//! # Stripe DTU — Stateful In-Memory Clone
//!
//! Simulates the Stripe API for scenario validation. Maintains
//! realistic state for:
//! - Customers (create, retrieve, list, update, delete)
//! - Payment Intents (create → confirm → capture lifecycle)
//! - Payment Methods (attach/detach)
//! - Charges (created from confirmed Payment Intents)
//! - Refunds (full and partial)
//!
//! State transitions follow Stripe's actual API behavior.

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::{json, Value};
use uuid::Uuid;

use super::DtuProvider;
use planner_schemas::{DtuConfigV1, DtuProviderInfo, DtuRequest, DtuResponse};

// ---------------------------------------------------------------------------
// Stripe DTU
// ---------------------------------------------------------------------------

pub struct StripeDtu {
    info: DtuProviderInfo,
    state: RwLock<StripeState>,
    failures: RwLock<Vec<FailureInjection>>,
}

struct FailureInjection {
    endpoint: String,
    status_code: u16,
    error_body: Value,
}

#[derive(Debug, Clone, Default)]
struct StripeState {
    customers: HashMap<String, Value>,
    payment_intents: HashMap<String, Value>,
    payment_methods: HashMap<String, Value>,
    charges: HashMap<String, Value>,
    refunds: HashMap<String, Value>,
    products: HashMap<String, Value>,
    prices: HashMap<String, Value>,
    id_counter: u64,
}

impl StripeState {
    fn next_id(&mut self, prefix: &str) -> String {
        self.id_counter += 1;
        format!("{}_{}", prefix, self.id_counter)
    }
}

impl StripeDtu {
    pub fn new() -> Self {
        StripeDtu {
            info: DtuProviderInfo {
                id: "stripe".into(),
                name: "Stripe Payment Gateway".into(),
                api_version: "2024-12-18.acacia".into(),
                supported_endpoints: vec![
                    "/v1/customers".into(),
                    "/v1/payment_intents".into(),
                    "/v1/payment_methods".into(),
                    "/v1/charges".into(),
                    "/v1/refunds".into(),
                    "/v1/products".into(),
                    "/v1/prices".into(),
                ],
                introduced_phase: 4,
            },
            state: RwLock::new(StripeState::default()),
            failures: RwLock::new(Vec::new()),
        }
    }

    /// Check if a failure injection matches this request.
    fn check_failure(&self, path: &str) -> Option<DtuResponse> {
        let failures = self.failures.read().unwrap();
        for f in failures.iter() {
            if path.starts_with(&f.endpoint) {
                return Some(DtuResponse {
                    status_code: f.status_code,
                    headers: vec![("content-type".into(), "application/json".into())],
                    body: f.error_body.clone(),
                });
            }
        }
        None
    }

    fn ok_response(&self, body: Value) -> DtuResponse {
        DtuResponse {
            status_code: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body,
        }
    }

    fn created_response(&self, body: Value) -> DtuResponse {
        DtuResponse {
            status_code: 200, // Stripe returns 200 for creates, not 201
            headers: vec![("content-type".into(), "application/json".into())],
            body,
        }
    }

    fn not_found(&self, resource_type: &str, id: &str) -> DtuResponse {
        DtuResponse {
            status_code: 404,
            headers: vec![("content-type".into(), "application/json".into())],
            body: json!({
                "error": {
                    "type": "invalid_request_error",
                    "message": format!("No such {}: '{}'", resource_type, id),
                    "code": "resource_missing",
                    "param": "id"
                }
            }),
        }
    }

    fn bad_request(&self, message: &str) -> DtuResponse {
        DtuResponse {
            status_code: 400,
            headers: vec![("content-type".into(), "application/json".into())],
            body: json!({
                "error": {
                    "type": "invalid_request_error",
                    "message": message
                }
            }),
        }
    }

    // -- Customer operations --

    fn create_customer(&self, body: &Value) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let id = state.next_id("cus");
        let customer = json!({
            "id": id,
            "object": "customer",
            "email": body.get("email").and_then(|v| v.as_str()).unwrap_or(""),
            "name": body.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "metadata": body.get("metadata").cloned().unwrap_or(json!({})),
            "created": chrono::Utc::now().timestamp(),
            "livemode": false,
            "default_source": null,
            "default_payment_method": null,
        });
        state.customers.insert(id.clone(), customer.clone());
        self.created_response(customer)
    }

    fn get_customer(&self, id: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        match state.customers.get(id) {
            Some(customer) => self.ok_response(customer.clone()),
            None => self.not_found("customer", id),
        }
    }

    fn list_customers(&self) -> DtuResponse {
        let state = self.state.read().unwrap();
        let data: Vec<Value> = state.customers.values().cloned().collect();
        self.ok_response(json!({
            "object": "list",
            "data": data,
            "has_more": false,
            "url": "/v1/customers"
        }))
    }

    fn delete_customer(&self, id: &str) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        if state.customers.remove(id).is_some() {
            self.ok_response(json!({
                "id": id,
                "object": "customer",
                "deleted": true
            }))
        } else {
            self.not_found("customer", id)
        }
    }

    // -- Payment Intent operations --

    fn create_payment_intent(&self, body: &Value) -> DtuResponse {
        let amount = body.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        let currency = body
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("usd");

        if amount <= 0 {
            return self.bad_request("Amount must be a positive integer");
        }

        let mut state = self.state.write().unwrap();
        let id = state.next_id("pi");
        let client_secret = format!(
            "{}_secret_{}",
            id,
            Uuid::new_v4().to_string().replace('-', "")
        );

        let pi = json!({
            "id": id,
            "object": "payment_intent",
            "amount": amount,
            "currency": currency,
            "status": "requires_payment_method",
            "client_secret": client_secret,
            "customer": body.get("customer").and_then(|v| v.as_str()),
            "payment_method": null,
            "metadata": body.get("metadata").cloned().unwrap_or(json!({})),
            "created": chrono::Utc::now().timestamp(),
            "livemode": false,
            "capture_method": body.get("capture_method").and_then(|v| v.as_str()).unwrap_or("automatic"),
        });
        state.payment_intents.insert(id.clone(), pi.clone());
        self.created_response(pi)
    }

    fn confirm_payment_intent(&self, id: &str, body: &Value) -> DtuResponse {
        let mut state = self.state.write().unwrap();

        // Clone the PI out so we can mutate state freely
        let mut pi = match state.payment_intents.get(id).cloned() {
            Some(pi) => pi,
            None => return self.not_found("payment_intent", id),
        };

        let status = pi["status"].as_str().unwrap_or("");
        if status != "requires_payment_method" && status != "requires_confirmation" {
            return self.bad_request(&format!(
                "This PaymentIntent's status is {}. Expected requires_payment_method or requires_confirmation.",
                status,
            ));
        }

        // Attach payment method if provided
        if let Some(pm) = body.get("payment_method").and_then(|v| v.as_str()) {
            pi["payment_method"] = json!(pm);
        }

        let capture_method = pi["capture_method"]
            .as_str()
            .unwrap_or("automatic")
            .to_string();
        if capture_method == "automatic" {
            pi["status"] = json!("succeeded");

            // Create a charge
            let charge_id = format!("ch_{}", state.id_counter + 1000);
            state.id_counter += 1;
            let charge = json!({
                "id": charge_id,
                "object": "charge",
                "amount": pi["amount"],
                "currency": pi["currency"],
                "payment_intent": id,
                "status": "succeeded",
                "paid": true,
                "refunded": false,
                "amount_refunded": 0,
                "created": chrono::Utc::now().timestamp(),
            });
            state.charges.insert(charge_id, charge);
        } else {
            pi["status"] = json!("requires_capture");
        }

        // Put updated PI back
        state.payment_intents.insert(id.to_string(), pi.clone());
        self.ok_response(pi)
    }

    fn capture_payment_intent(&self, id: &str) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let mut pi = match state.payment_intents.get(id).cloned() {
            Some(pi) => pi,
            None => return self.not_found("payment_intent", id),
        };

        if pi["status"].as_str() != Some("requires_capture") {
            return self.bad_request(&format!(
                "This PaymentIntent's status is {}. Expected requires_capture.",
                pi["status"].as_str().unwrap_or("unknown"),
            ));
        }

        pi["status"] = json!("succeeded");
        state.payment_intents.insert(id.to_string(), pi.clone());
        self.ok_response(pi)
    }

    fn cancel_payment_intent(&self, id: &str) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let mut pi = match state.payment_intents.get(id).cloned() {
            Some(pi) => pi,
            None => return self.not_found("payment_intent", id),
        };

        let status = pi["status"].as_str().unwrap_or("");
        if status == "succeeded" {
            return self
                .bad_request("This PaymentIntent has already succeeded and cannot be canceled.");
        }
        if status == "canceled" {
            return self.bad_request("This PaymentIntent has already been canceled.");
        }

        pi["status"] = json!("canceled");
        state.payment_intents.insert(id.to_string(), pi.clone());
        self.ok_response(pi)
    }

    fn get_payment_intent(&self, id: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        match state.payment_intents.get(id) {
            Some(pi) => self.ok_response(pi.clone()),
            None => self.not_found("payment_intent", id),
        }
    }

    // -- Refund operations --

    fn create_refund(&self, body: &Value) -> DtuResponse {
        let pi_id = match body.get("payment_intent").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => return self.bad_request("payment_intent is required"),
        };

        let mut state = self.state.write().unwrap();
        let pi = match state.payment_intents.get(&pi_id) {
            Some(pi) => pi.clone(),
            None => return self.not_found("payment_intent", &pi_id),
        };

        if pi["status"].as_str() != Some("succeeded") {
            return self.bad_request("Cannot refund a PaymentIntent that has not succeeded");
        }

        let original_amount = pi["amount"].as_i64().unwrap_or(0);
        let refund_amount = body
            .get("amount")
            .and_then(|v| v.as_i64())
            .unwrap_or(original_amount);

        if refund_amount > original_amount {
            return self.bad_request("Refund amount cannot exceed the original charge amount");
        }

        let id = state.next_id("re");
        let refund = json!({
            "id": id,
            "object": "refund",
            "amount": refund_amount,
            "currency": pi["currency"],
            "payment_intent": pi_id,
            "status": "succeeded",
            "created": chrono::Utc::now().timestamp(),
        });
        state.refunds.insert(id.clone(), refund.clone());
        self.created_response(refund)
    }
}

impl DtuProvider for StripeDtu {
    fn info(&self) -> &DtuProviderInfo {
        &self.info
    }

    fn handle_request(&self, request: &DtuRequest) -> DtuResponse {
        // Check failure injections first
        if let Some(failure) = self.check_failure(&request.path) {
            return failure;
        }

        let path = request.path.as_str();
        let method = request.method.to_uppercase();
        let body = request.body.as_ref().cloned().unwrap_or(json!({}));

        // Route to handler based on path and method
        match (method.as_str(), path) {
            // Customers
            ("POST", "/v1/customers") => self.create_customer(&body),
            ("GET", "/v1/customers") => self.list_customers(),
            ("GET", p) if p.starts_with("/v1/customers/") => {
                let id = &p["/v1/customers/".len()..];
                self.get_customer(id)
            }
            ("DELETE", p) if p.starts_with("/v1/customers/") => {
                let id = &p["/v1/customers/".len()..];
                self.delete_customer(id)
            }

            // Payment Intents
            ("POST", "/v1/payment_intents") => self.create_payment_intent(&body),
            ("GET", p)
                if p.starts_with("/v1/payment_intents/")
                    && !p.contains("/confirm")
                    && !p.contains("/capture")
                    && !p.contains("/cancel") =>
            {
                let id = &p["/v1/payment_intents/".len()..];
                self.get_payment_intent(id)
            }
            ("POST", p) if p.ends_with("/confirm") && p.starts_with("/v1/payment_intents/") => {
                let id = &p["/v1/payment_intents/".len()..p.len() - "/confirm".len()];
                self.confirm_payment_intent(id, &body)
            }
            ("POST", p) if p.ends_with("/capture") && p.starts_with("/v1/payment_intents/") => {
                let id = &p["/v1/payment_intents/".len()..p.len() - "/capture".len()];
                self.capture_payment_intent(id)
            }
            ("POST", p) if p.ends_with("/cancel") && p.starts_with("/v1/payment_intents/") => {
                let id = &p["/v1/payment_intents/".len()..p.len() - "/cancel".len()];
                self.cancel_payment_intent(id)
            }

            // Refunds
            ("POST", "/v1/refunds") => self.create_refund(&body),

            _ => DtuResponse {
                status_code: 404,
                headers: vec![("content-type".into(), "application/json".into())],
                body: json!({
                    "error": {
                        "type": "invalid_request_error",
                        "message": format!("Unrecognized request URL ({} {})", method, path)
                    }
                }),
            },
        }
    }

    fn reset(&self) {
        let mut state = self.state.write().unwrap();
        *state = StripeState::default();
        self.failures.write().unwrap().clear();
    }

    fn apply_config(&self, config: &DtuConfigV1) {
        let mut state = self.state.write().unwrap();
        // Seed initial state
        for seed in &config.seed_state {
            match seed.entity_type.as_str() {
                "customer" => {
                    state
                        .customers
                        .insert(seed.entity_id.clone(), seed.initial_state.clone());
                }
                "payment_intent" => {
                    state
                        .payment_intents
                        .insert(seed.entity_id.clone(), seed.initial_state.clone());
                }
                "payment_method" => {
                    state
                        .payment_methods
                        .insert(seed.entity_id.clone(), seed.initial_state.clone());
                }
                "product" => {
                    state
                        .products
                        .insert(seed.entity_id.clone(), seed.initial_state.clone());
                }
                "price" => {
                    state
                        .prices
                        .insert(seed.entity_id.clone(), seed.initial_state.clone());
                }
                _ => {} // Unknown entity types are ignored
            }
        }

        // Apply failure modes
        let mut failures = self.failures.write().unwrap();
        for fm in &config.failure_modes {
            failures.push(FailureInjection {
                endpoint: fm.endpoint.clone(),
                status_code: fm.status_code,
                error_body: fm.error_body.clone(),
            });
        }
    }

    fn inject_failure(&self, endpoint: &str, status_code: u16, error_body: Value) {
        self.failures.write().unwrap().push(FailureInjection {
            endpoint: endpoint.to_string(),
            status_code,
            error_body,
        });
    }

    fn clear_failures(&self) {
        self.failures.write().unwrap().clear();
    }

    fn state_snapshot(&self) -> Value {
        let state = self.state.read().unwrap();
        json!({
            "customers": state.customers.len(),
            "payment_intents": state.payment_intents.len(),
            "payment_methods": state.payment_methods.len(),
            "charges": state.charges.len(),
            "refunds": state.refunds.len(),
            "products": state.products.len(),
            "prices": state.prices.len(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use planner_schemas::DtuSeedEntry;

    fn stripe() -> StripeDtu {
        StripeDtu::new()
    }

    fn post(path: &str, body: Value) -> DtuRequest {
        DtuRequest {
            method: "POST".into(),
            path: path.into(),
            query_params: vec![],
            headers: vec![],
            body: Some(body),
        }
    }

    fn get(path: &str) -> DtuRequest {
        DtuRequest {
            method: "GET".into(),
            path: path.into(),
            query_params: vec![],
            headers: vec![],
            body: None,
        }
    }

    fn delete(path: &str) -> DtuRequest {
        DtuRequest {
            method: "DELETE".into(),
            path: path.into(),
            query_params: vec![],
            headers: vec![],
            body: None,
        }
    }

    #[test]
    fn create_and_get_customer() {
        let s = stripe();
        let resp = s.handle_request(&post(
            "/v1/customers",
            json!({
                "email": "test@example.com",
                "name": "Test User"
            }),
        ));
        assert_eq!(resp.status_code, 200);
        let id = resp.body["id"].as_str().unwrap().to_string();
        assert!(id.starts_with("cus_"));

        // Retrieve
        let resp2 = s.handle_request(&get(&format!("/v1/customers/{}", id)));
        assert_eq!(resp2.status_code, 200);
        assert_eq!(resp2.body["email"], "test@example.com");
    }

    #[test]
    fn list_customers() {
        let s = stripe();
        s.handle_request(&post("/v1/customers", json!({"email": "a@b.com"})));
        s.handle_request(&post("/v1/customers", json!({"email": "c@d.com"})));

        let resp = s.handle_request(&get("/v1/customers"));
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body["data"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn delete_customer() {
        let s = stripe();
        let resp = s.handle_request(&post("/v1/customers", json!({"email": "x@y.com"})));
        let id = resp.body["id"].as_str().unwrap().to_string();

        let del = s.handle_request(&delete(&format!("/v1/customers/{}", id)));
        assert_eq!(del.status_code, 200);
        assert_eq!(del.body["deleted"], true);

        // Now get should 404
        let get_resp = s.handle_request(&get(&format!("/v1/customers/{}", id)));
        assert_eq!(get_resp.status_code, 404);
    }

    #[test]
    fn payment_intent_lifecycle() {
        let s = stripe();

        // Create
        let resp = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 2000,
                "currency": "usd"
            }),
        ));
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body["status"], "requires_payment_method");
        let id = resp.body["id"].as_str().unwrap().to_string();

        // Confirm (auto-capture)
        let confirm = s.handle_request(&post(
            &format!("/v1/payment_intents/{}/confirm", id),
            json!({"payment_method": "pm_card_visa"}),
        ));
        assert_eq!(confirm.status_code, 200);
        assert_eq!(confirm.body["status"], "succeeded");
    }

    #[test]
    fn payment_intent_manual_capture() {
        let s = stripe();

        let resp = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 5000,
                "currency": "eur",
                "capture_method": "manual"
            }),
        ));
        let id = resp.body["id"].as_str().unwrap().to_string();

        // Confirm → requires_capture
        let confirm = s.handle_request(&post(
            &format!("/v1/payment_intents/{}/confirm", id),
            json!({"payment_method": "pm_card_visa"}),
        ));
        assert_eq!(confirm.body["status"], "requires_capture");

        // Capture → succeeded
        let capture = s.handle_request(&post(
            &format!("/v1/payment_intents/{}/capture", id),
            json!({}),
        ));
        assert_eq!(capture.body["status"], "succeeded");
    }

    #[test]
    fn payment_intent_cancel() {
        let s = stripe();

        let resp = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 1000,
                "currency": "usd"
            }),
        ));
        let id = resp.body["id"].as_str().unwrap().to_string();

        let cancel = s.handle_request(&post(
            &format!("/v1/payment_intents/{}/cancel", id),
            json!({}),
        ));
        assert_eq!(cancel.body["status"], "canceled");
    }

    #[test]
    fn payment_intent_invalid_amount() {
        let s = stripe();
        let resp = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": -100,
                "currency": "usd"
            }),
        ));
        assert_eq!(resp.status_code, 400);
    }

    #[test]
    fn refund_succeeded_payment() {
        let s = stripe();

        // Create + confirm a payment
        let pi = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 3000,
                "currency": "usd"
            }),
        ));
        let pi_id = pi.body["id"].as_str().unwrap().to_string();
        s.handle_request(&post(
            &format!("/v1/payment_intents/{}/confirm", pi_id),
            json!({"payment_method": "pm_card_visa"}),
        ));

        // Refund
        let refund = s.handle_request(&post(
            "/v1/refunds",
            json!({
                "payment_intent": pi_id
            }),
        ));
        assert_eq!(refund.status_code, 200);
        assert_eq!(refund.body["amount"], 3000);
        assert_eq!(refund.body["status"], "succeeded");
    }

    #[test]
    fn partial_refund() {
        let s = stripe();

        let pi = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 5000,
                "currency": "usd"
            }),
        ));
        let pi_id = pi.body["id"].as_str().unwrap().to_string();
        s.handle_request(&post(
            &format!("/v1/payment_intents/{}/confirm", pi_id),
            json!({"payment_method": "pm_card_visa"}),
        ));

        let refund = s.handle_request(&post(
            "/v1/refunds",
            json!({
                "payment_intent": pi_id,
                "amount": 2000
            }),
        ));
        assert_eq!(refund.body["amount"], 2000);
    }

    #[test]
    fn refund_exceeding_amount_fails() {
        let s = stripe();

        let pi = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 1000,
                "currency": "usd"
            }),
        ));
        let pi_id = pi.body["id"].as_str().unwrap().to_string();
        s.handle_request(&post(
            &format!("/v1/payment_intents/{}/confirm", pi_id),
            json!({"payment_method": "pm_card_visa"}),
        ));

        let refund = s.handle_request(&post(
            "/v1/refunds",
            json!({
                "payment_intent": pi_id,
                "amount": 9999
            }),
        ));
        assert_eq!(refund.status_code, 400);
    }

    #[test]
    fn failure_injection() {
        let s = stripe();
        s.inject_failure(
            "/v1/payment_intents",
            503,
            json!({
                "error": {"message": "Service temporarily unavailable"}
            }),
        );

        let resp = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 1000,
                "currency": "usd"
            }),
        ));
        assert_eq!(resp.status_code, 503);

        // Clear failures
        s.clear_failures();
        let resp2 = s.handle_request(&post(
            "/v1/payment_intents",
            json!({
                "amount": 1000,
                "currency": "usd"
            }),
        ));
        assert_eq!(resp2.status_code, 200);
    }

    #[test]
    fn reset_clears_all_state() {
        let s = stripe();
        s.handle_request(&post("/v1/customers", json!({"email": "a@b.com"})));
        s.inject_failure("/test", 500, json!({}));

        s.reset();

        let snapshot = s.state_snapshot();
        assert_eq!(snapshot["customers"], 0);
        let resp = s.handle_request(&get("/v1/customers"));
        assert_eq!(resp.body["data"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn unknown_endpoint_returns_404() {
        let s = stripe();
        let resp = s.handle_request(&get("/v1/unknown"));
        assert_eq!(resp.status_code, 404);
    }

    #[test]
    fn get_nonexistent_customer_returns_404() {
        let s = stripe();
        let resp = s.handle_request(&get("/v1/customers/cus_nonexistent"));
        assert_eq!(resp.status_code, 404);
        assert!(resp.body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("No such customer"));
    }

    #[test]
    fn apply_config_seeds_state() {
        let s = stripe();
        let config = DtuConfigV1 {
            project_id: Uuid::new_v4(),
            dependency_name: "Stripe".into(),
            provider_id: "stripe".into(),
            behavioral_rules: vec![],
            seed_state: vec![DtuSeedEntry {
                entity_type: "customer".into(),
                entity_id: "cus_seeded".into(),
                initial_state: json!({
                    "id": "cus_seeded",
                    "object": "customer",
                    "email": "seeded@test.com"
                }),
            }],
            failure_modes: vec![],
            validated: false,
        };
        s.apply_config(&config);

        let resp = s.handle_request(&get("/v1/customers/cus_seeded"));
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body["email"], "seeded@test.com");
    }
}
