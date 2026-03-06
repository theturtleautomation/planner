//! # Twilio DTU — Stateful In-Memory Clone
//!
//! Emulates Twilio's messaging and voice APIs.
//!
//! ## Supported Operations
//! - `POST .../Messages.json` — Send SMS/MMS
//! - `GET .../Messages.json` — List messages
//! - `GET .../Messages/:sid.json` — Get message by SID
//! - `POST .../Calls.json` — Initiate call
//! - `GET .../Calls/:sid.json` — Get call status
//!
//! ## State Model
//! Messages: `queued` → `sent` → `delivered` | `failed`
//! Calls: `queued` → `ringing` → `in-progress` → `completed` | `failed`

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::{json, Value};

use super::DtuProvider;
use planner_schemas::{DtuConfigV1, DtuProviderInfo, DtuRequest, DtuResponse};

// ---------------------------------------------------------------------------
// Twilio DTU Clone
// ---------------------------------------------------------------------------

pub struct TwilioDtu {
    info: DtuProviderInfo,
    state: RwLock<TwilioState>,
    failures: RwLock<Vec<FailureInjection>>,
}

struct FailureInjection {
    endpoint: String,
    status_code: u16,
    error_body: Value,
}

#[derive(Debug, Clone, Default)]
struct TwilioState {
    messages: HashMap<String, SmsMessage>,
    calls: HashMap<String, VoiceCall>,
    sid_counter: u64,
}

#[derive(Debug, Clone)]
struct SmsMessage {
    sid: String,
    from: String,
    to: String,
    body: String,
    status: String,
    num_segments: u32,
}

#[derive(Debug, Clone)]
struct VoiceCall {
    sid: String,
    from: String,
    to: String,
    url: String,
    status: String,
}

impl TwilioState {
    fn next_message_sid(&mut self) -> String {
        self.sid_counter += 1;
        format!("SM{:032x}", self.sid_counter)
    }

    fn next_call_sid(&mut self) -> String {
        self.sid_counter += 1;
        format!("CA{:032x}", self.sid_counter)
    }
}

impl TwilioDtu {
    pub fn new() -> Self {
        TwilioDtu {
            info: DtuProviderInfo {
                id: "twilio".into(),
                name: "Twilio".into(),
                api_version: "2010-04-01".into(),
                supported_endpoints: vec![
                    "POST .../Messages.json".into(),
                    "GET .../Messages.json".into(),
                    "GET .../Messages/:sid.json".into(),
                    "POST .../Calls.json".into(),
                    "GET .../Calls/:sid.json".into(),
                ],
                introduced_phase: 5,
            },
            state: RwLock::new(TwilioState::default()),
            failures: RwLock::new(Vec::new()),
        }
    }

    fn check_failure(&self, path: &str) -> Option<DtuResponse> {
        let failures = self.failures.read().unwrap();
        for f in failures.iter() {
            if path.contains(&f.endpoint) {
                return Some(DtuResponse {
                    status_code: f.status_code,
                    headers: vec![],
                    body: f.error_body.clone(),
                });
            }
        }
        None
    }

    /// Parse Twilio-style paths.
    /// e.g. "/2010-04-01/Accounts/AC.../Messages.json" → ("Messages", None)
    /// e.g. "/2010-04-01/Accounts/AC.../Messages/SM123.json" → ("Messages", Some("SM123"))
    fn parse_path(path: &str) -> (String, Option<String>) {
        let stripped = path.trim_end_matches(".json");
        let parts: Vec<&str> = stripped.split('/').collect();

        for (i, part) in parts.iter().enumerate() {
            if *part == "Messages" || *part == "Calls" {
                let resource = part.to_string();
                let sid = parts.get(i + 1).map(|s| s.to_string());
                return (resource, sid);
            }
        }
        ("Unknown".into(), None)
    }

    fn send_message(&self, body: &Value) -> DtuResponse {
        let from = body
            .get("From")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let to = body
            .get("To")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let msg_body = body
            .get("Body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if from.is_empty() || to.is_empty() {
            return DtuResponse {
                status_code: 400,
                headers: vec![],
                body: json!({"code": 21211, "message": "'To' and 'From' are required", "status": 400}),
            };
        }

        let mut state = self.state.write().unwrap();
        let sid = state.next_message_sid();
        let num_segments = (msg_body.len() as u32 / 160) + 1;

        state.messages.insert(
            sid.clone(),
            SmsMessage {
                sid: sid.clone(),
                from: from.clone(),
                to: to.clone(),
                body: msg_body.clone(),
                status: "sent".into(),
                num_segments,
            },
        );

        DtuResponse {
            status_code: 201,
            headers: vec![],
            body: json!({
                "sid": sid, "from": from, "to": to, "body": msg_body,
                "status": "sent", "num_segments": num_segments.to_string(),
                "price": "-0.0075", "price_unit": "USD",
            }),
        }
    }

    fn list_messages(&self) -> DtuResponse {
        let state = self.state.read().unwrap();
        let messages: Vec<Value> = state.messages.values().map(|m| {
            json!({"sid": m.sid, "from": m.from, "to": m.to, "body": m.body, "status": m.status})
        }).collect();

        DtuResponse {
            status_code: 200,
            headers: vec![],
            body: json!({"messages": messages, "page": 0}),
        }
    }

    fn get_message(&self, sid: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        match state.messages.get(sid) {
            Some(m) => DtuResponse {
                status_code: 200,
                headers: vec![],
                body: json!({"sid": m.sid, "from": m.from, "to": m.to, "body": m.body, "status": m.status, "num_segments": m.num_segments.to_string()}),
            },
            None => DtuResponse {
                status_code: 404,
                headers: vec![],
                body: json!({"code": 20404, "message": "Message not found", "status": 404}),
            },
        }
    }

    fn create_call(&self, body: &Value) -> DtuResponse {
        let from = body
            .get("From")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let to = body
            .get("To")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let url = body
            .get("Url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if from.is_empty() || to.is_empty() {
            return DtuResponse {
                status_code: 400,
                headers: vec![],
                body: json!({"code": 21211, "message": "'To' and 'From' are required", "status": 400}),
            };
        }

        let mut state = self.state.write().unwrap();
        let sid = state.next_call_sid();
        state.calls.insert(
            sid.clone(),
            VoiceCall {
                sid: sid.clone(),
                from: from.clone(),
                to: to.clone(),
                url: url.clone(),
                status: "queued".into(),
            },
        );

        DtuResponse {
            status_code: 201,
            headers: vec![],
            body: json!({"sid": sid, "from": from, "to": to, "url": url, "status": "queued"}),
        }
    }

    fn get_call(&self, sid: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        match state.calls.get(sid) {
            Some(c) => DtuResponse {
                status_code: 200,
                headers: vec![],
                body: json!({"sid": c.sid, "from": c.from, "to": c.to, "url": c.url, "status": c.status}),
            },
            None => DtuResponse {
                status_code: 404,
                headers: vec![],
                body: json!({"code": 20404, "message": "Call not found", "status": 404}),
            },
        }
    }
}

impl DtuProvider for TwilioDtu {
    fn info(&self) -> &DtuProviderInfo {
        &self.info
    }

    fn handle_request(&self, request: &DtuRequest) -> DtuResponse {
        if let Some(failure) = self.check_failure(&request.path) {
            return failure;
        }

        let (resource, sid) = Self::parse_path(&request.path);
        let method = request.method.to_uppercase();
        let body = request.body.as_ref().cloned().unwrap_or(json!({}));

        match (method.as_str(), resource.as_str(), sid.as_deref()) {
            ("POST", "Messages", None) => self.send_message(&body),
            ("GET", "Messages", None) => self.list_messages(),
            ("GET", "Messages", Some(msg_sid)) => self.get_message(msg_sid),
            ("POST", "Calls", None) => self.create_call(&body),
            ("GET", "Calls", Some(call_sid)) => self.get_call(call_sid),
            _ => DtuResponse {
                status_code: 404,
                headers: vec![],
                body: json!({"code": 20404, "message": "Resource not found", "status": 404}),
            },
        }
    }

    fn reset(&self) {
        *self.state.write().unwrap() = TwilioState::default();
        self.failures.write().unwrap().clear();
    }

    fn apply_config(&self, config: &DtuConfigV1) {
        let mut state = self.state.write().unwrap();
        for seed in &config.seed_state {
            match seed.entity_type.as_str() {
                "message" => {
                    state.messages.insert(
                        seed.entity_id.clone(),
                        SmsMessage {
                            sid: seed.entity_id.clone(),
                            from: seed
                                .initial_state
                                .get("from")
                                .and_then(|v| v.as_str())
                                .unwrap_or("+15551234567")
                                .to_string(),
                            to: seed
                                .initial_state
                                .get("to")
                                .and_then(|v| v.as_str())
                                .unwrap_or("+15559876543")
                                .to_string(),
                            body: seed
                                .initial_state
                                .get("body")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Seeded")
                                .to_string(),
                            status: "delivered".into(),
                            num_segments: 1,
                        },
                    );
                }
                "call" => {
                    state.calls.insert(
                        seed.entity_id.clone(),
                        VoiceCall {
                            sid: seed.entity_id.clone(),
                            from: seed
                                .initial_state
                                .get("from")
                                .and_then(|v| v.as_str())
                                .unwrap_or("+15551234567")
                                .to_string(),
                            to: seed
                                .initial_state
                                .get("to")
                                .and_then(|v| v.as_str())
                                .unwrap_or("+15559876543")
                                .to_string(),
                            url: seed
                                .initial_state
                                .get("url")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            status: "completed".into(),
                        },
                    );
                }
                _ => {}
            }
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
            "message_count": state.messages.len(),
            "call_count": state.calls.len(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn messages_path() -> String {
        "/2010-04-01/Accounts/AC_test/Messages.json".into()
    }

    fn calls_path() -> String {
        "/2010-04-01/Accounts/AC_test/Calls.json".into()
    }

    #[test]
    fn send_sms() {
        let dtu = TwilioDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: Some(json!({"From": "+15551234567", "To": "+15559876543", "Body": "Hello!"})),
        });
        assert_eq!(resp.status_code, 201);
        assert!(resp.body.get("sid").is_some());
        assert_eq!(
            resp.body.get("status").and_then(|v| v.as_str()).unwrap(),
            "sent"
        );
    }

    #[test]
    fn send_sms_missing_fields_400() {
        let dtu = TwilioDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: Some(json!({"Body": "No from/to"})),
        });
        assert_eq!(resp.status_code, 400);
    }

    #[test]
    fn list_messages() {
        let dtu = TwilioDtu::new();
        for to in &["+15551111111", "+15552222222"] {
            dtu.handle_request(&DtuRequest {
                method: "POST".into(),
                path: messages_path(),
                query_params: vec![],
                headers: vec![],
                body: Some(json!({"From": "+15550000000", "To": to, "Body": "Test"})),
            });
        }

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: None,
        });
        assert_eq!(
            resp.body
                .get("messages")
                .and_then(|m| m.as_array())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn get_message_by_sid() {
        let dtu = TwilioDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: Some(json!({"From": "+15550000000", "To": "+15551111111", "Body": "Hello"})),
        });
        let sid = resp
            .body
            .get("sid")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(),
            path: format!("/2010-04-01/Accounts/AC_test/Messages/{}.json", sid),
            query_params: vec![],
            headers: vec![],
            body: None,
        });
        assert_eq!(resp.status_code, 200);
        assert_eq!(
            resp.body.get("body").and_then(|v| v.as_str()).unwrap(),
            "Hello"
        );
    }

    #[test]
    fn create_call() {
        let dtu = TwilioDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: calls_path(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"From": "+15550000000", "To": "+15551111111", "Url": "http://example.com/twiml"})),
        });
        assert_eq!(resp.status_code, 201);
        assert_eq!(
            resp.body.get("status").and_then(|v| v.as_str()).unwrap(),
            "queued"
        );
    }

    #[test]
    fn get_call_by_sid() {
        let dtu = TwilioDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: calls_path(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"From": "+15550000000", "To": "+15551111111", "Url": "http://example.com/twiml"})),
        });
        let sid = resp
            .body
            .get("sid")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(),
            path: format!("/2010-04-01/Accounts/AC_test/Calls/{}.json", sid),
            query_params: vec![],
            headers: vec![],
            body: None,
        });
        assert_eq!(resp.status_code, 200);
    }

    #[test]
    fn failure_injection() {
        let dtu = TwilioDtu::new();
        dtu.inject_failure(
            "Messages",
            429,
            json!({"code": 20429, "message": "Too many requests"}),
        );

        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: Some(json!({"From": "+15550000000", "To": "+15551111111", "Body": "Test"})),
        });
        assert_eq!(resp.status_code, 429);
    }

    #[test]
    fn message_segmentation() {
        let dtu = TwilioDtu::new();
        let long_body = "a".repeat(320);
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: Some(json!({"From": "+15550000000", "To": "+15551111111", "Body": long_body})),
        });
        assert_eq!(
            resp.body
                .get("num_segments")
                .and_then(|v| v.as_str())
                .unwrap(),
            "3"
        );
    }

    #[test]
    fn reset_clears_all_state() {
        let dtu = TwilioDtu::new();
        dtu.handle_request(&DtuRequest {
            method: "POST".into(),
            path: messages_path(),
            query_params: vec![],
            headers: vec![],
            body: Some(json!({"From": "+15550000000", "To": "+15551111111", "Body": "Test"})),
        });
        dtu.reset();
        assert!(dtu.state.read().unwrap().messages.is_empty());
    }

    #[test]
    fn parse_twilio_path_messages() {
        let (resource, sid) = TwilioDtu::parse_path("/2010-04-01/Accounts/AC_test/Messages.json");
        assert_eq!(resource, "Messages");
        assert!(sid.is_none());
    }

    #[test]
    fn parse_twilio_path_message_with_sid() {
        let (resource, sid) =
            TwilioDtu::parse_path("/2010-04-01/Accounts/AC_test/Messages/SM123.json");
        assert_eq!(resource, "Messages");
        assert_eq!(sid.unwrap(), "SM123");
    }

    #[test]
    fn parse_twilio_path_calls() {
        let (resource, sid) = TwilioDtu::parse_path("/2010-04-01/Accounts/AC_test/Calls.json");
        assert_eq!(resource, "Calls");
        assert!(sid.is_none());
    }

    #[test]
    fn get_nonexistent_message_404() {
        let dtu = TwilioDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(),
            path: "/2010-04-01/Accounts/AC_test/Messages/SM_NOPE.json".into(),
            query_params: vec![],
            headers: vec![],
            body: None,
        });
        assert_eq!(resp.status_code, 404);
    }
}
