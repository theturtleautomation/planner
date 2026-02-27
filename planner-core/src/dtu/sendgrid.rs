//! # SendGrid DTU — Stateful In-Memory Clone
//!
//! Emulates SendGrid's transactional email API (v3).
//!
//! ## Supported Operations
//! - `POST /v3/mail/send` — Send transactional email
//! - `GET /v3/messages` — List sent messages (activity feed)
//! - `GET /v3/messages/:msg_id` — Get message status
//!
//! ## State Model
//! Emails transition through: `queued` → `processed` → `delivered` | `bounced` | `dropped`

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::{json, Value};

use planner_schemas::{DtuConfigV1, DtuProviderInfo, DtuRequest, DtuResponse};
use super::DtuProvider;

// ---------------------------------------------------------------------------
// SendGrid DTU Clone
// ---------------------------------------------------------------------------

pub struct SendGridDtu {
    info: DtuProviderInfo,
    state: RwLock<SendGridState>,
    failures: RwLock<Vec<FailureInjection>>,
}

struct FailureInjection {
    endpoint: String,
    status_code: u16,
    error_body: Value,
}

#[derive(Debug, Clone, Default)]
struct SendGridState {
    messages: HashMap<String, EmailMessage>,
    msg_counter: u64,
}

#[derive(Debug, Clone)]
struct EmailMessage {
    msg_id: String,
    from: String,
    to: Vec<String>,
    subject: String,
    status: String,
}

impl SendGridState {
    fn next_msg_id(&mut self) -> String {
        self.msg_counter += 1;
        format!("sg_msg_{}", self.msg_counter)
    }
}

impl SendGridDtu {
    pub fn new() -> Self {
        SendGridDtu {
            info: DtuProviderInfo {
                id: "sendgrid".into(),
                name: "SendGrid Email".into(),
                api_version: "v3".into(),
                supported_endpoints: vec![
                    "/v3/mail/send".into(),
                    "/v3/messages".into(),
                    "/v3/messages/:msg_id".into(),
                ],
                introduced_phase: 5,
            },
            state: RwLock::new(SendGridState::default()),
            failures: RwLock::new(Vec::new()),
        }
    }

    fn check_failure(&self, path: &str) -> Option<DtuResponse> {
        let failures = self.failures.read().unwrap();
        for f in failures.iter() {
            if path.starts_with(&f.endpoint) {
                return Some(DtuResponse {
                    status_code: f.status_code,
                    headers: vec![],
                    body: f.error_body.clone(),
                });
            }
        }
        None
    }

    fn send_mail(&self, body: &Value) -> DtuResponse {
        let to_emails = extract_recipients(body);
        if to_emails.is_empty() {
            return DtuResponse {
                status_code: 400,
                headers: vec![],
                body: json!({"errors": [{"message": "No recipients specified", "field": "personalizations"}]}),
            };
        }

        let from = body.pointer("/from/email")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown@example.com")
            .to_string();
        let subject = body.get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("(no subject)")
            .to_string();

        let mut state = self.state.write().unwrap();
        let msg_id = state.next_msg_id();

        let message = EmailMessage {
            msg_id: msg_id.clone(),
            from,
            to: to_emails,
            subject,
            status: "processed".into(),
        };
        state.messages.insert(msg_id.clone(), message);

        DtuResponse {
            status_code: 202,
            headers: vec![("X-Message-Id".into(), msg_id)],
            body: json!(null),
        }
    }

    fn list_messages(&self) -> DtuResponse {
        let state = self.state.read().unwrap();
        let messages: Vec<Value> = state.messages.values().map(|m| {
            json!({
                "msg_id": m.msg_id,
                "from_email": m.from,
                "to_email": m.to.first().unwrap_or(&String::new()),
                "subject": m.subject,
                "status": m.status,
            })
        }).collect();

        DtuResponse {
            status_code: 200,
            headers: vec![],
            body: json!({ "messages": messages }),
        }
    }

    fn get_message(&self, msg_id: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        match state.messages.get(msg_id) {
            Some(m) => DtuResponse {
                status_code: 200,
                headers: vec![],
                body: json!({
                    "msg_id": m.msg_id,
                    "from_email": m.from,
                    "to_email": m.to,
                    "subject": m.subject,
                    "status": m.status,
                }),
            },
            None => DtuResponse {
                status_code: 404,
                headers: vec![],
                body: json!({"errors": [{"message": "Message not found"}]}),
            },
        }
    }
}

fn extract_recipients(body: &Value) -> Vec<String> {
    body.get("personalizations")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter().flat_map(|p| {
                p.get("to")
                    .and_then(|t| t.as_array())
                    .map(|to_arr| {
                        to_arr.iter()
                            .filter_map(|r| r.get("email").and_then(|e| e.as_str()))
                            .map(String::from)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            }).collect()
        })
        .unwrap_or_default()
}

impl DtuProvider for SendGridDtu {
    fn info(&self) -> &DtuProviderInfo { &self.info }

    fn handle_request(&self, request: &DtuRequest) -> DtuResponse {
        if let Some(failure) = self.check_failure(&request.path) {
            return failure;
        }

        let path = request.path.as_str();
        let method = request.method.to_uppercase();
        let body = request.body.as_ref().cloned().unwrap_or(json!({}));

        match (method.as_str(), path) {
            ("POST", "/v3/mail/send") => self.send_mail(&body),
            ("GET", "/v3/messages") => self.list_messages(),
            ("GET", p) if p.starts_with("/v3/messages/") => {
                let msg_id = &p["/v3/messages/".len()..];
                self.get_message(msg_id)
            }
            _ => DtuResponse {
                status_code: 404,
                headers: vec![],
                body: json!({"errors": [{"message": "Not found"}]}),
            },
        }
    }

    fn reset(&self) {
        *self.state.write().unwrap() = SendGridState::default();
        self.failures.write().unwrap().clear();
    }

    fn apply_config(&self, config: &DtuConfigV1) {
        let mut state = self.state.write().unwrap();
        for seed in &config.seed_state {
            if seed.entity_type == "message" {
                let msg = EmailMessage {
                    msg_id: seed.entity_id.clone(),
                    from: seed.initial_state.get("from").and_then(|v| v.as_str()).unwrap_or("seed@example.com").to_string(),
                    to: vec![seed.initial_state.get("to").and_then(|v| v.as_str()).unwrap_or("test@example.com").to_string()],
                    subject: seed.initial_state.get("subject").and_then(|v| v.as_str()).unwrap_or("Seeded").to_string(),
                    status: "delivered".into(),
                };
                state.messages.insert(msg.msg_id.clone(), msg);
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
            "message_ids": state.messages.keys().collect::<Vec<_>>(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn send_mail_body(to: &str, subject: &str) -> Value {
        json!({
            "personalizations": [{"to": [{"email": to}]}],
            "from": {"email": "sender@example.com"},
            "subject": subject,
            "content": [{"type": "text/html", "value": "<p>Hello</p>"}]
        })
    }

    fn make_send_request(body: Value) -> DtuRequest {
        DtuRequest {
            method: "POST".into(),
            path: "/v3/mail/send".into(),
            query_params: vec![], headers: vec![],
            body: Some(body),
        }
    }

    #[test]
    fn send_mail_returns_202() {
        let dtu = SendGridDtu::new();
        let resp = dtu.handle_request(&make_send_request(send_mail_body("test@example.com", "Hello")));
        assert_eq!(resp.status_code, 202);
        assert!(resp.headers.iter().any(|(k, _)| k == "X-Message-Id"));
    }

    #[test]
    fn send_mail_no_recipients_400() {
        let dtu = SendGridDtu::new();
        let resp = dtu.handle_request(&make_send_request(json!({"from": {"email": "a@b.com"}})));
        assert_eq!(resp.status_code, 400);
    }

    #[test]
    fn list_messages_after_send() {
        let dtu = SendGridDtu::new();
        dtu.handle_request(&make_send_request(send_mail_body("test@example.com", "Hello")));

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/v3/messages".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 200);
        let messages = resp.body.get("messages").and_then(|m| m.as_array()).unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn get_message_by_id() {
        let dtu = SendGridDtu::new();
        let send_resp = dtu.handle_request(&make_send_request(send_mail_body("test@example.com", "Test Email")));
        let msg_id = send_resp.headers.iter()
            .find(|(k, _)| k == "X-Message-Id")
            .map(|(_, v)| v.clone())
            .unwrap();

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(),
            path: format!("/v3/messages/{}", msg_id),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body.get("subject").and_then(|v| v.as_str()).unwrap(), "Test Email");
    }

    #[test]
    fn get_nonexistent_message_404() {
        let dtu = SendGridDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/v3/messages/nonexistent".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 404);
    }

    #[test]
    fn failure_injection() {
        let dtu = SendGridDtu::new();
        dtu.inject_failure("/v3/mail/send", 429, json!({"errors": [{"message": "Rate limit"}]}));

        let resp = dtu.handle_request(&make_send_request(send_mail_body("test@example.com", "Test")));
        assert_eq!(resp.status_code, 429);
    }

    #[test]
    fn reset_clears_all_state() {
        let dtu = SendGridDtu::new();
        dtu.handle_request(&make_send_request(send_mail_body("test@example.com", "Hello")));
        assert_eq!(dtu.state.read().unwrap().messages.len(), 1);

        dtu.reset();
        assert!(dtu.state.read().unwrap().messages.is_empty());
    }

    #[test]
    fn unknown_endpoint_404() {
        let dtu = SendGridDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/v3/unknown".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 404);
    }
}
