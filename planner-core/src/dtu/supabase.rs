//! # Supabase DTU — Stateful In-Memory Clone
//!
//! Emulates Supabase's PostgREST API and Auth endpoints.
//!
//! ## Supported Operations
//! - `POST /auth/v1/signup` — Create user account
//! - `POST /auth/v1/token` — Login (password grant)
//! - `GET /auth/v1/user` — Get current authenticated user
//! - `GET /rest/v1/:table` — Read rows (with eq filter support)
//! - `POST /rest/v1/:table` — Insert row(s)
//! - `DELETE /rest/v1/:table?col.eq=val` — Delete matching rows

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::{json, Value};
use uuid::Uuid;

use planner_schemas::{DtuConfigV1, DtuProviderInfo, DtuRequest, DtuResponse};
use super::DtuProvider;

// ---------------------------------------------------------------------------
// Supabase DTU Clone
// ---------------------------------------------------------------------------

pub struct SupabaseDtu {
    info: DtuProviderInfo,
    state: RwLock<SupabaseState>,
    failures: RwLock<Vec<FailureInjection>>,
}

struct FailureInjection {
    endpoint: String,
    status_code: u16,
    error_body: Value,
}

#[derive(Debug, Clone, Default)]
struct SupabaseState {
    tables: HashMap<String, Vec<Value>>,
    users: HashMap<String, UserRecord>,
    tokens: HashMap<String, String>, // token → user_id
}

#[derive(Debug, Clone)]
struct UserRecord {
    id: String,
    email: String,
    password: String,
    metadata: Value,
}

impl SupabaseDtu {
    pub fn new() -> Self {
        SupabaseDtu {
            info: DtuProviderInfo {
                id: "supabase".into(),
                name: "Supabase".into(),
                api_version: "v1".into(),
                supported_endpoints: vec![
                    "/auth/v1/signup".into(),
                    "/auth/v1/token".into(),
                    "/auth/v1/user".into(),
                    "/rest/v1/:table".into(),
                ],
                introduced_phase: 5,
            },
            state: RwLock::new(SupabaseState::default()),
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

    fn signup(&self, body: &Value) -> DtuResponse {
        let email = body.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("").to_string();

        if email.is_empty() || password.is_empty() {
            return DtuResponse {
                status_code: 400, headers: vec![],
                body: json!({"error": "email and password required"}),
            };
        }

        let mut state = self.state.write().unwrap();

        if state.users.values().any(|u| u.email == email) {
            return DtuResponse {
                status_code: 422, headers: vec![],
                body: json!({"error": "User already registered"}),
            };
        }

        let user_id = Uuid::new_v4().to_string();
        let token = format!("sb_token_{}", Uuid::new_v4().to_string().replace('-', ""));

        state.users.insert(user_id.clone(), UserRecord {
            id: user_id.clone(),
            email: email.clone(),
            password,
            metadata: body.get("data").cloned().unwrap_or(json!({})),
        });
        state.tokens.insert(token.clone(), user_id.clone());

        DtuResponse {
            status_code: 200, headers: vec![],
            body: json!({
                "access_token": token, "token_type": "bearer",
                "user": {"id": user_id, "email": email, "role": "authenticated"}
            }),
        }
    }

    fn login(&self, body: &Value) -> DtuResponse {
        let email = body.get("email").and_then(|v| v.as_str()).unwrap_or("");
        let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");

        let mut state = self.state.write().unwrap();
        let user = state.users.values().find(|u| u.email == email && u.password == password).cloned();

        match user {
            Some(u) => {
                let token = format!("sb_token_{}", Uuid::new_v4().to_string().replace('-', ""));
                state.tokens.insert(token.clone(), u.id.clone());
                DtuResponse {
                    status_code: 200, headers: vec![],
                    body: json!({
                        "access_token": token, "token_type": "bearer",
                        "user": {"id": u.id, "email": u.email, "role": "authenticated"}
                    }),
                }
            }
            None => DtuResponse {
                status_code: 400, headers: vec![],
                body: json!({"error": "Invalid login credentials"}),
            },
        }
    }

    fn get_user(&self, headers: &[(String, String)]) -> DtuResponse {
        let token = headers.iter()
            .find(|(k, _)| k.to_lowercase() == "authorization")
            .map(|(_, v)| v.strip_prefix("Bearer ").unwrap_or(v).to_string());

        let state = self.state.read().unwrap();
        let user = token
            .and_then(|t| state.tokens.get(&t).cloned())
            .and_then(|uid| state.users.get(&uid).cloned());

        match user {
            Some(u) => DtuResponse {
                status_code: 200, headers: vec![],
                body: json!({"id": u.id, "email": u.email, "user_metadata": u.metadata}),
            },
            None => DtuResponse {
                status_code: 401, headers: vec![],
                body: json!({"error": "Not authenticated"}),
            },
        }
    }

    fn table_read(&self, table: &str, query_params: &[(String, String)]) -> DtuResponse {
        let state = self.state.read().unwrap();
        let rows = state.tables.get(table).cloned().unwrap_or_default();

        let mut filtered = rows;
        for (key, value) in query_params {
            if let Some(col) = key.strip_suffix(".eq") {
                filtered.retain(|row| {
                    row.get(col).and_then(|v| v.as_str()) == Some(value.as_str())
                });
            }
        }

        DtuResponse { status_code: 200, headers: vec![], body: json!(filtered) }
    }

    fn table_insert(&self, table: &str, body: &Value) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let rows = state.tables.entry(table.to_string()).or_default();

        if let Some(arr) = body.as_array() {
            for row in arr {
                let mut r = row.clone();
                if r.get("id").is_none() {
                    r.as_object_mut().map(|o| o.insert("id".into(), json!(Uuid::new_v4().to_string())));
                }
                rows.push(r);
            }
        } else {
            let mut r = body.clone();
            if r.get("id").is_none() {
                r.as_object_mut().map(|o| o.insert("id".into(), json!(Uuid::new_v4().to_string())));
            }
            rows.push(r);
        }

        DtuResponse { status_code: 201, headers: vec![], body: body.clone() }
    }

    fn table_delete(&self, table: &str, query_params: &[(String, String)]) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        if let Some(rows) = state.tables.get_mut(table) {
            let before = rows.len();
            for (key, value) in query_params {
                if let Some(col) = key.strip_suffix(".eq") {
                    rows.retain(|row| {
                        row.get(col).and_then(|v| v.as_str()) != Some(value.as_str())
                    });
                }
            }
            DtuResponse { status_code: 200, headers: vec![], body: json!({"deleted": before - rows.len()}) }
        } else {
            DtuResponse { status_code: 200, headers: vec![], body: json!({"deleted": 0}) }
        }
    }
}

impl DtuProvider for SupabaseDtu {
    fn info(&self) -> &DtuProviderInfo { &self.info }

    fn handle_request(&self, request: &DtuRequest) -> DtuResponse {
        if let Some(failure) = self.check_failure(&request.path) {
            return failure;
        }

        let path = request.path.as_str();
        let method = request.method.to_uppercase();
        let body = request.body.as_ref().cloned().unwrap_or(json!({}));

        match (method.as_str(), path) {
            ("POST", "/auth/v1/signup") => self.signup(&body),
            ("POST", "/auth/v1/token") => self.login(&body),
            ("GET", "/auth/v1/user") => self.get_user(&request.headers),
            ("GET", p) if p.starts_with("/rest/v1/") => {
                let table = &p["/rest/v1/".len()..];
                self.table_read(table, &request.query_params)
            }
            ("POST", p) if p.starts_with("/rest/v1/") => {
                let table = p["/rest/v1/".len()..].to_string();
                self.table_insert(&table, &body)
            }
            ("DELETE", p) if p.starts_with("/rest/v1/") => {
                let table = p["/rest/v1/".len()..].to_string();
                self.table_delete(&table, &request.query_params)
            }
            _ => DtuResponse {
                status_code: 404, headers: vec![],
                body: json!({"error": "Not found"}),
            },
        }
    }

    fn reset(&self) {
        *self.state.write().unwrap() = SupabaseState::default();
        self.failures.write().unwrap().clear();
    }

    fn apply_config(&self, config: &DtuConfigV1) {
        let mut state = self.state.write().unwrap();
        for seed in &config.seed_state {
            match seed.entity_type.as_str() {
                "user" => {
                    state.users.insert(seed.entity_id.clone(), UserRecord {
                        id: seed.entity_id.clone(),
                        email: seed.initial_state.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        password: seed.initial_state.get("password").and_then(|v| v.as_str()).unwrap_or("password").to_string(),
                        metadata: json!({}),
                    });
                }
                "row" => {
                    let table = seed.initial_state.get("_table").and_then(|v| v.as_str()).unwrap_or("default").to_string();
                    state.tables.entry(table).or_default().push(seed.initial_state.clone());
                }
                _ => {}
            }
        }
    }

    fn inject_failure(&self, endpoint: &str, status_code: u16, error_body: Value) {
        self.failures.write().unwrap().push(FailureInjection {
            endpoint: endpoint.to_string(), status_code, error_body,
        });
    }

    fn clear_failures(&self) {
        self.failures.write().unwrap().clear();
    }

    fn state_snapshot(&self) -> Value {
        let state = self.state.read().unwrap();
        json!({
            "user_count": state.users.len(),
            "table_count": state.tables.len(),
            "tables": state.tables.keys().collect::<Vec<_>>(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signup_and_login() {
        let dtu = SupabaseDtu::new();

        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/signup".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"email": "test@example.com", "password": "secret123"})),
        });
        assert_eq!(resp.status_code, 200);
        assert!(resp.body.get("access_token").is_some());

        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/token".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"email": "test@example.com", "password": "secret123"})),
        });
        assert_eq!(resp.status_code, 200);
        let token = resp.body.get("access_token").and_then(|v| v.as_str()).unwrap().to_string();

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/auth/v1/user".into(),
            query_params: vec![],
            headers: vec![("Authorization".into(), format!("Bearer {}", token))],
            body: None,
        });
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body.get("email").and_then(|v| v.as_str()).unwrap(), "test@example.com");
    }

    #[test]
    fn duplicate_signup_fails() {
        let dtu = SupabaseDtu::new();
        let body = json!({"email": "test@example.com", "password": "secret"});

        dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/signup".into(),
            query_params: vec![], headers: vec![], body: Some(body.clone()),
        });

        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/signup".into(),
            query_params: vec![], headers: vec![], body: Some(body),
        });
        assert_eq!(resp.status_code, 422);
    }

    #[test]
    fn wrong_password_fails() {
        let dtu = SupabaseDtu::new();
        dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/signup".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"email": "test@example.com", "password": "correct"})),
        });

        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/token".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"email": "test@example.com", "password": "wrong"})),
        });
        assert_eq!(resp.status_code, 400);
    }

    #[test]
    fn table_crud() {
        let dtu = SupabaseDtu::new();

        // Insert
        let resp = dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/rest/v1/todos".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"title": "Buy milk", "done": false})),
        });
        assert_eq!(resp.status_code, 201);

        // Read
        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/rest/v1/todos".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body.as_array().unwrap().len(), 1);
    }

    #[test]
    fn table_read_with_filter() {
        let dtu = SupabaseDtu::new();

        for title in &["Task A", "Task B"] {
            dtu.handle_request(&DtuRequest {
                method: "POST".into(), path: "/rest/v1/tasks".into(),
                query_params: vec![], headers: vec![],
                body: Some(json!({"title": title, "status": if *title == "Task A" { "done" } else { "pending" }})),
            });
        }

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/rest/v1/tasks".into(),
            query_params: vec![("status.eq".into(), "done".into())],
            headers: vec![], body: None,
        });
        assert_eq!(resp.body.as_array().unwrap().len(), 1);
    }

    #[test]
    fn table_delete() {
        let dtu = SupabaseDtu::new();

        dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/rest/v1/items".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"name": "Widget", "sku": "W001"})),
        });

        dtu.handle_request(&DtuRequest {
            method: "DELETE".into(), path: "/rest/v1/items".into(),
            query_params: vec![("sku.eq".into(), "W001".into())],
            headers: vec![], body: None,
        });

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/rest/v1/items".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert!(resp.body.as_array().unwrap().is_empty());
    }

    #[test]
    fn unauthenticated_user_401() {
        let dtu = SupabaseDtu::new();
        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/auth/v1/user".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 401);
    }

    #[test]
    fn reset_clears_all_state() {
        let dtu = SupabaseDtu::new();
        dtu.handle_request(&DtuRequest {
            method: "POST".into(), path: "/auth/v1/signup".into(),
            query_params: vec![], headers: vec![],
            body: Some(json!({"email": "test@example.com", "password": "secret"})),
        });
        dtu.reset();
        assert!(dtu.state.read().unwrap().users.is_empty());
    }

    #[test]
    fn failure_injection() {
        let dtu = SupabaseDtu::new();
        dtu.inject_failure("/rest/v1/", 500, json!({"error": "Internal server error"}));

        let resp = dtu.handle_request(&DtuRequest {
            method: "GET".into(), path: "/rest/v1/todos".into(),
            query_params: vec![], headers: vec![], body: None,
        });
        assert_eq!(resp.status_code, 500);
    }
}
