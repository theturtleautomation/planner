//! # Auth0 DTU — Stateful In-Memory Clone
//!
//! Simulates the Auth0 Management and Authentication APIs for scenario
//! validation. Maintains realistic state for:
//! - Users (create, retrieve, update, delete, list)
//! - Token issuance (login → JWT generation)
//! - Token validation and refresh
//! - Password reset flow
//! - Roles and permissions (basic RBAC)
//!
//! JWT tokens are deterministic test tokens (not cryptographically signed).

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::{json, Value};

use planner_schemas::{DtuConfigV1, DtuProviderInfo, DtuRequest, DtuResponse};
use super::DtuProvider;

// ---------------------------------------------------------------------------
// Auth0 DTU
// ---------------------------------------------------------------------------

pub struct Auth0Dtu {
    info: DtuProviderInfo,
    state: RwLock<Auth0State>,
    failures: RwLock<Vec<FailureInjection>>,
}

struct FailureInjection {
    endpoint: String,
    status_code: u16,
    error_body: Value,
}

#[derive(Debug, Clone, Default)]
struct Auth0State {
    /// Users keyed by user_id.
    users: HashMap<String, Value>,
    /// Active tokens keyed by access_token string.
    tokens: HashMap<String, TokenInfo>,
    /// Roles keyed by role_id.
    roles: HashMap<String, Value>,
    /// User-role assignments: user_id → Vec<role_id>.
    user_roles: HashMap<String, Vec<String>>,
    /// Password reset tokens: token → user_id.
    reset_tokens: HashMap<String, String>,
    id_counter: u64,
    token_counter: u64,
}

#[derive(Debug, Clone)]
struct TokenInfo {
    user_id: String,
    access_token: String,
    refresh_token: String,
    id_token: String,
    expires_in: u64,
    scope: String,
    created_at: i64,
}

impl Auth0State {
    fn next_id(&mut self, prefix: &str) -> String {
        self.id_counter += 1;
        format!("{}|{}", prefix, self.id_counter)
    }

    fn next_token(&mut self) -> String {
        self.token_counter += 1;
        format!("test_token_{}", self.token_counter)
    }
}

impl Auth0Dtu {
    pub fn new() -> Self {
        Auth0Dtu {
            info: DtuProviderInfo {
                id: "auth0".into(),
                name: "Auth0 Identity Platform".into(),
                api_version: "v2".into(),
                supported_endpoints: vec![
                    "/oauth/token".into(),
                    "/api/v2/users".into(),
                    "/api/v2/roles".into(),
                    "/dbconnections/change_password".into(),
                    "/userinfo".into(),
                    "/api/v2/users-by-email".into(),
                ],
                introduced_phase: 4,
            },
            state: RwLock::new(Auth0State::default()),
            failures: RwLock::new(Vec::new()),
        }
    }

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
            status_code: 201,
            headers: vec![("content-type".into(), "application/json".into())],
            body,
        }
    }

    fn error_response(&self, status_code: u16, error: &str, message: &str) -> DtuResponse {
        DtuResponse {
            status_code,
            headers: vec![("content-type".into(), "application/json".into())],
            body: json!({
                "statusCode": status_code,
                "error": error,
                "message": message
            }),
        }
    }

    // -- Authentication endpoints --

    fn oauth_token(&self, body: &Value) -> DtuResponse {
        let grant_type = body.get("grant_type").and_then(|v| v.as_str()).unwrap_or("");

        match grant_type {
            "password" => self.password_grant(body),
            "refresh_token" => self.refresh_token_grant(body),
            "client_credentials" => self.client_credentials_grant(body),
            _ => self.error_response(400, "invalid_grant", &format!("Unknown grant_type: {}", grant_type)),
        }
    }

    fn password_grant(&self, body: &Value) -> DtuResponse {
        let email = body.get("username").and_then(|v| v.as_str()).unwrap_or("");
        let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");

        let state = self.state.read().unwrap();

        // Find user by email
        let user = state.users.values().find(|u| {
            u.get("email").and_then(|e| e.as_str()) == Some(email)
        });

        let user = match user {
            Some(u) => u.clone(),
            None => return self.error_response(403, "invalid_grant", "Wrong email or password"),
        };

        // Check password (stored in plaintext in the mock — this is a test clone)
        let stored_password = user.get("_password").and_then(|p| p.as_str()).unwrap_or("");
        if stored_password != password {
            return self.error_response(403, "invalid_grant", "Wrong email or password");
        }

        let user_id = user["user_id"].as_str().unwrap_or("").to_string();
        drop(state);

        self.issue_tokens(&user_id, "openid profile email")
    }

    fn refresh_token_grant(&self, body: &Value) -> DtuResponse {
        let refresh_token = body.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("");

        let state = self.state.read().unwrap();
        let token_info = state.tokens.values().find(|t| t.refresh_token == refresh_token);

        match token_info {
            Some(info) => {
                let user_id = info.user_id.clone();
                drop(state);
                self.issue_tokens(&user_id, "openid profile email")
            }
            None => self.error_response(403, "invalid_grant", "Invalid refresh token"),
        }
    }

    fn client_credentials_grant(&self, _body: &Value) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let access_token = state.next_token();
        self.ok_response(json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": 86400,
            "scope": "read:users update:users"
        }))
    }

    fn issue_tokens(&self, user_id: &str, scope: &str) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let access_token = state.next_token();
        let refresh_token = format!("refresh_{}", state.next_token());
        let id_token = format!("eyJ.test_id_token.{}", &access_token);

        let info = TokenInfo {
            user_id: user_id.to_string(),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
            id_token: id_token.clone(),
            expires_in: 86400,
            scope: scope.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };

        state.tokens.insert(access_token.clone(), info);

        self.ok_response(json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "id_token": id_token,
            "token_type": "Bearer",
            "expires_in": 86400,
            "scope": scope
        }))
    }

    fn userinfo(&self, request: &DtuRequest) -> DtuResponse {
        // Extract bearer token from Authorization header
        let auth_header = request.headers.iter()
            .find(|(k, _)| k.to_lowercase() == "authorization")
            .map(|(_, v)| v.as_str());

        let token = match auth_header {
            Some(h) if h.starts_with("Bearer ") => &h[7..],
            _ => return self.error_response(401, "Unauthorized", "Missing or invalid authorization header"),
        };

        let state = self.state.read().unwrap();
        let token_info = match state.tokens.get(token) {
            Some(info) => info,
            None => return self.error_response(401, "Unauthorized", "Invalid access token"),
        };

        let user = match state.users.get(&token_info.user_id) {
            Some(u) => u,
            None => return self.error_response(404, "Not Found", "User not found"),
        };

        self.ok_response(json!({
            "sub": user["user_id"],
            "email": user["email"],
            "email_verified": user.get("email_verified").unwrap_or(&json!(false)),
            "name": user.get("name").unwrap_or(&json!("")),
            "nickname": user.get("nickname").unwrap_or(&json!("")),
            "picture": user.get("picture").unwrap_or(&json!("")),
        }))
    }

    // -- Management API: Users --

    fn create_user(&self, body: &Value) -> DtuResponse {
        let email = match body.get("email").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => return self.error_response(400, "Bad Request", "email is required"),
        };

        let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("default_password");
        let connection = body.get("connection").and_then(|v| v.as_str()).unwrap_or("Username-Password-Authentication");

        let mut state = self.state.write().unwrap();

        // Check duplicate email
        if state.users.values().any(|u| u.get("email").and_then(|e| e.as_str()) == Some(email)) {
            return self.error_response(409, "Conflict", "The user already exists");
        }

        let user_id = state.next_id("auth0");
        let user = json!({
            "user_id": user_id,
            "email": email,
            "email_verified": body.get("email_verified").unwrap_or(&json!(false)),
            "name": body.get("name").unwrap_or(&json!(email)),
            "nickname": body.get("nickname").unwrap_or(&json!(email.split('@').next().unwrap_or(""))),
            "connection": connection,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "blocked": false,
            "_password": password, // Internal only — not returned in responses
        });
        state.users.insert(user_id.clone(), user.clone());

        // Return without _password
        let mut response_user = user;
        response_user.as_object_mut().unwrap().remove("_password");
        self.created_response(response_user)
    }

    fn get_user(&self, user_id: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        match state.users.get(user_id) {
            Some(user) => {
                let mut u = user.clone();
                u.as_object_mut().unwrap().remove("_password");
                self.ok_response(u)
            }
            None => self.error_response(404, "Not Found", &format!("User not found: {}", user_id)),
        }
    }

    fn list_users(&self) -> DtuResponse {
        let state = self.state.read().unwrap();
        let users: Vec<Value> = state.users.values().map(|u| {
            let mut user = u.clone();
            user.as_object_mut().unwrap().remove("_password");
            user
        }).collect();
        self.ok_response(json!(users))
    }

    fn update_user(&self, user_id: &str, body: &Value) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        let user = match state.users.get_mut(user_id) {
            Some(u) => u,
            None => return self.error_response(404, "Not Found", &format!("User not found: {}", user_id)),
        };

        // Apply updates
        if let Some(obj) = body.as_object() {
            for (key, value) in obj {
                if key == "password" {
                    user["_password"] = value.clone();
                } else {
                    user[key] = value.clone();
                }
            }
        }
        user["updated_at"] = json!(chrono::Utc::now().to_rfc3339());

        let mut response_user = user.clone();
        response_user.as_object_mut().unwrap().remove("_password");
        self.ok_response(response_user)
    }

    fn delete_user(&self, user_id: &str) -> DtuResponse {
        let mut state = self.state.write().unwrap();
        if state.users.remove(user_id).is_some() {
            state.user_roles.remove(user_id);
            DtuResponse {
                status_code: 204,
                headers: vec![],
                body: json!(null),
            }
        } else {
            self.error_response(404, "Not Found", &format!("User not found: {}", user_id))
        }
    }

    fn users_by_email(&self, request: &DtuRequest) -> DtuResponse {
        let email = request.query_params.iter()
            .find(|(k, _)| k == "email")
            .map(|(_, v)| v.as_str());

        let email = match email {
            Some(e) => e,
            None => return self.error_response(400, "Bad Request", "email query parameter is required"),
        };

        let state = self.state.read().unwrap();
        let users: Vec<Value> = state.users.values()
            .filter(|u| u.get("email").and_then(|e| e.as_str()) == Some(email))
            .map(|u| {
                let mut user = u.clone();
                user.as_object_mut().unwrap().remove("_password");
                user
            })
            .collect();

        self.ok_response(json!(users))
    }

    // -- Password reset --

    fn change_password(&self, body: &Value) -> DtuResponse {
        let email = match body.get("email").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => return self.error_response(400, "Bad Request", "email is required"),
        };

        let state = self.state.read().unwrap();
        let user = state.users.values().find(|u| {
            u.get("email").and_then(|e| e.as_str()) == Some(email)
        });

        if user.is_none() {
            // Auth0 returns 200 even for unknown emails (security best practice)
            return self.ok_response(json!("We've just sent you an email to reset your password."));
        }

        let user_id = user.unwrap()["user_id"].as_str().unwrap().to_string();
        drop(state);

        let mut state = self.state.write().unwrap();
        let token = format!("reset_{}", state.next_token());
        state.reset_tokens.insert(token, user_id);

        self.ok_response(json!("We've just sent you an email to reset your password."))
    }

    // -- Roles --

    fn create_role(&self, body: &Value) -> DtuResponse {
        let name = match body.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return self.error_response(400, "Bad Request", "name is required"),
        };

        let mut state = self.state.write().unwrap();
        let role_id = state.next_id("rol");
        let role = json!({
            "id": role_id,
            "name": name,
            "description": body.get("description").unwrap_or(&json!("")),
        });
        state.roles.insert(role_id.clone(), role.clone());
        self.created_response(role)
    }

    fn list_roles(&self) -> DtuResponse {
        let state = self.state.read().unwrap();
        let roles: Vec<Value> = state.roles.values().cloned().collect();
        self.ok_response(json!(roles))
    }

    fn assign_roles(&self, user_id: &str, body: &Value) -> DtuResponse {
        let roles = body.get("roles").and_then(|v| v.as_array());
        let roles = match roles {
            Some(r) => r.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>(),
            None => return self.error_response(400, "Bad Request", "roles array is required"),
        };

        let mut state = self.state.write().unwrap();
        if !state.users.contains_key(user_id) {
            return self.error_response(404, "Not Found", "User not found");
        }

        let user_roles = state.user_roles.entry(user_id.to_string()).or_default();
        for role_id in roles {
            if !user_roles.contains(&role_id) {
                user_roles.push(role_id);
            }
        }

        DtuResponse {
            status_code: 204,
            headers: vec![],
            body: json!(null),
        }
    }

    fn get_user_roles(&self, user_id: &str) -> DtuResponse {
        let state = self.state.read().unwrap();
        if !state.users.contains_key(user_id) {
            return self.error_response(404, "Not Found", "User not found");
        }

        let role_ids = state.user_roles.get(user_id)
            .cloned()
            .unwrap_or_default();

        let roles: Vec<Value> = role_ids.iter()
            .filter_map(|rid| state.roles.get(rid).cloned())
            .collect();

        self.ok_response(json!(roles))
    }
}

impl DtuProvider for Auth0Dtu {
    fn info(&self) -> &DtuProviderInfo {
        &self.info
    }

    fn handle_request(&self, request: &DtuRequest) -> DtuResponse {
        if let Some(failure) = self.check_failure(&request.path) {
            return failure;
        }

        let path = request.path.as_str();
        let method = request.method.to_uppercase();
        let body = request.body.as_ref().cloned().unwrap_or(json!({}));

        match (method.as_str(), path) {
            // Authentication
            ("POST", "/oauth/token") => self.oauth_token(&body),
            ("GET", "/userinfo") => self.userinfo(request),
            ("POST", "/dbconnections/change_password") => self.change_password(&body),

            // Management API: Users
            ("POST", "/api/v2/users") => self.create_user(&body),
            ("GET", "/api/v2/users") => self.list_users(),
            ("GET", "/api/v2/users-by-email") => self.users_by_email(request),
            ("GET", p) if p.starts_with("/api/v2/users/") && !p.contains("/roles") => {
                let user_id = &p["/api/v2/users/".len()..];
                self.get_user(user_id)
            }
            ("PATCH", p) if p.starts_with("/api/v2/users/") && !p.contains("/roles") => {
                let user_id = &p["/api/v2/users/".len()..];
                self.update_user(user_id, &body)
            }
            ("DELETE", p) if p.starts_with("/api/v2/users/") => {
                let user_id = &p["/api/v2/users/".len()..];
                self.delete_user(user_id)
            }

            // Roles
            ("POST", "/api/v2/roles") => self.create_role(&body),
            ("GET", "/api/v2/roles") => self.list_roles(),
            ("POST", p) if p.starts_with("/api/v2/users/") && p.ends_with("/roles") => {
                let user_id = &p["/api/v2/users/".len()..p.len() - "/roles".len()];
                self.assign_roles(user_id, &body)
            }
            ("GET", p) if p.starts_with("/api/v2/users/") && p.ends_with("/roles") => {
                let user_id = &p["/api/v2/users/".len()..p.len() - "/roles".len()];
                self.get_user_roles(user_id)
            }

            _ => DtuResponse {
                status_code: 404,
                headers: vec![("content-type".into(), "application/json".into())],
                body: json!({
                    "statusCode": 404,
                    "error": "Not Found",
                    "message": format!("Route {} {} not found", method, path)
                }),
            },
        }
    }

    fn reset(&self) {
        *self.state.write().unwrap() = Auth0State::default();
        self.failures.write().unwrap().clear();
    }

    fn apply_config(&self, config: &DtuConfigV1) {
        let mut state = self.state.write().unwrap();
        for seed in &config.seed_state {
            match seed.entity_type.as_str() {
                "user" => { state.users.insert(seed.entity_id.clone(), seed.initial_state.clone()); }
                "role" => { state.roles.insert(seed.entity_id.clone(), seed.initial_state.clone()); }
                _ => {}
            }
        }

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
            "users": state.users.len(),
            "tokens": state.tokens.len(),
            "roles": state.roles.len(),
            "reset_tokens": state.reset_tokens.len(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn auth0() -> Auth0Dtu {
        Auth0Dtu::new()
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

    fn get_with_headers(path: &str, headers: Vec<(String, String)>) -> DtuRequest {
        DtuRequest {
            method: "GET".into(),
            path: path.into(),
            query_params: vec![],
            headers,
            body: None,
        }
    }

    fn get_with_query(path: &str, params: Vec<(String, String)>) -> DtuRequest {
        DtuRequest {
            method: "GET".into(),
            path: path.into(),
            query_params: params,
            headers: vec![],
            body: None,
        }
    }

    fn patch(path: &str, body: Value) -> DtuRequest {
        DtuRequest {
            method: "PATCH".into(),
            path: path.into(),
            query_params: vec![],
            headers: vec![],
            body: Some(body),
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
    fn create_and_get_user() {
        let a = auth0();
        let resp = a.handle_request(&post("/api/v2/users", json!({
            "email": "test@example.com",
            "password": "Password123!",
            "connection": "Username-Password-Authentication"
        })));
        assert_eq!(resp.status_code, 201);
        let user_id = resp.body["user_id"].as_str().unwrap().to_string();
        assert!(user_id.starts_with("auth0|"));
        // Password should not be in response
        assert!(resp.body.get("_password").is_none());

        // Retrieve
        let resp2 = a.handle_request(&get(&format!("/api/v2/users/{}", user_id)));
        assert_eq!(resp2.status_code, 200);
        assert_eq!(resp2.body["email"], "test@example.com");
    }

    #[test]
    fn duplicate_email_fails() {
        let a = auth0();
        a.handle_request(&post("/api/v2/users", json!({
            "email": "dup@test.com",
            "password": "Pass123!"
        })));

        let resp = a.handle_request(&post("/api/v2/users", json!({
            "email": "dup@test.com",
            "password": "Pass456!"
        })));
        assert_eq!(resp.status_code, 409);
    }

    #[test]
    fn password_login_flow() {
        let a = auth0();

        // Create user
        a.handle_request(&post("/api/v2/users", json!({
            "email": "login@test.com",
            "password": "MyPassword123"
        })));

        // Login
        let resp = a.handle_request(&post("/oauth/token", json!({
            "grant_type": "password",
            "username": "login@test.com",
            "password": "MyPassword123"
        })));
        assert_eq!(resp.status_code, 200);
        assert!(resp.body.get("access_token").is_some());
        assert!(resp.body.get("refresh_token").is_some());
        assert!(resp.body.get("id_token").is_some());
        assert_eq!(resp.body["token_type"], "Bearer");
    }

    #[test]
    fn wrong_password_fails() {
        let a = auth0();

        a.handle_request(&post("/api/v2/users", json!({
            "email": "wrong@test.com",
            "password": "CorrectPassword"
        })));

        let resp = a.handle_request(&post("/oauth/token", json!({
            "grant_type": "password",
            "username": "wrong@test.com",
            "password": "WrongPassword"
        })));
        assert_eq!(resp.status_code, 403);
    }

    #[test]
    fn userinfo_with_valid_token() {
        let a = auth0();

        a.handle_request(&post("/api/v2/users", json!({
            "email": "info@test.com",
            "password": "Pass123"
        })));

        let login = a.handle_request(&post("/oauth/token", json!({
            "grant_type": "password",
            "username": "info@test.com",
            "password": "Pass123"
        })));
        let token = login.body["access_token"].as_str().unwrap().to_string();

        let resp = a.handle_request(&get_with_headers(
            "/userinfo",
            vec![("Authorization".into(), format!("Bearer {}", token))],
        ));
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body["email"], "info@test.com");
    }

    #[test]
    fn userinfo_without_token_fails() {
        let a = auth0();
        let resp = a.handle_request(&get("/userinfo"));
        assert_eq!(resp.status_code, 401);
    }

    #[test]
    fn refresh_token_grant() {
        let a = auth0();

        a.handle_request(&post("/api/v2/users", json!({
            "email": "refresh@test.com",
            "password": "Pass123"
        })));

        let login = a.handle_request(&post("/oauth/token", json!({
            "grant_type": "password",
            "username": "refresh@test.com",
            "password": "Pass123"
        })));
        let refresh = login.body["refresh_token"].as_str().unwrap().to_string();

        let resp = a.handle_request(&post("/oauth/token", json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh
        })));
        assert_eq!(resp.status_code, 200);
        assert!(resp.body.get("access_token").is_some());
    }

    #[test]
    fn update_user() {
        let a = auth0();
        let create = a.handle_request(&post("/api/v2/users", json!({
            "email": "update@test.com",
            "password": "Pass123"
        })));
        let user_id = create.body["user_id"].as_str().unwrap().to_string();

        let resp = a.handle_request(&patch(
            &format!("/api/v2/users/{}", user_id),
            json!({"name": "Updated Name"}),
        ));
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body["name"], "Updated Name");
    }

    #[test]
    fn delete_user() {
        let a = auth0();
        let create = a.handle_request(&post("/api/v2/users", json!({
            "email": "delete@test.com",
            "password": "Pass123"
        })));
        let user_id = create.body["user_id"].as_str().unwrap().to_string();

        let resp = a.handle_request(&delete(&format!("/api/v2/users/{}", user_id)));
        assert_eq!(resp.status_code, 204);

        // Should be gone
        let get_resp = a.handle_request(&get(&format!("/api/v2/users/{}", user_id)));
        assert_eq!(get_resp.status_code, 404);
    }

    #[test]
    fn role_assignment() {
        let a = auth0();

        // Create user + role
        let user = a.handle_request(&post("/api/v2/users", json!({
            "email": "role@test.com",
            "password": "Pass123"
        })));
        let user_id = user.body["user_id"].as_str().unwrap().to_string();

        let role = a.handle_request(&post("/api/v2/roles", json!({
            "name": "admin",
            "description": "Administrator"
        })));
        let role_id = role.body["id"].as_str().unwrap().to_string();

        // Assign role
        let assign = a.handle_request(&post(
            &format!("/api/v2/users/{}/roles", user_id),
            json!({"roles": [role_id]}),
        ));
        assert_eq!(assign.status_code, 204);

        // Check roles
        let roles_resp = a.handle_request(&get(&format!("/api/v2/users/{}/roles", user_id)));
        assert_eq!(roles_resp.status_code, 200);
        let roles_arr = roles_resp.body.as_array().unwrap();
        assert_eq!(roles_arr.len(), 1);
        assert_eq!(roles_arr[0]["name"], "admin");
    }

    #[test]
    fn users_by_email() {
        let a = auth0();
        a.handle_request(&post("/api/v2/users", json!({
            "email": "find@test.com",
            "password": "Pass123"
        })));

        let resp = a.handle_request(&get_with_query(
            "/api/v2/users-by-email",
            vec![("email".into(), "find@test.com".into())],
        ));
        assert_eq!(resp.status_code, 200);
        let users = resp.body.as_array().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0]["email"], "find@test.com");
    }

    #[test]
    fn password_reset() {
        let a = auth0();
        a.handle_request(&post("/api/v2/users", json!({
            "email": "reset@test.com",
            "password": "Pass123"
        })));

        let resp = a.handle_request(&post("/dbconnections/change_password", json!({
            "email": "reset@test.com",
            "connection": "Username-Password-Authentication"
        })));
        assert_eq!(resp.status_code, 200);

        // Even for unknown emails, should return 200
        let resp2 = a.handle_request(&post("/dbconnections/change_password", json!({
            "email": "unknown@test.com"
        })));
        assert_eq!(resp2.status_code, 200);
    }

    #[test]
    fn failure_injection() {
        let a = auth0();
        a.inject_failure("/oauth/token", 429, json!({
            "error": "too_many_requests",
            "message": "Rate limit exceeded"
        }));

        let resp = a.handle_request(&post("/oauth/token", json!({
            "grant_type": "password",
            "username": "test",
            "password": "test"
        })));
        assert_eq!(resp.status_code, 429);
    }

    #[test]
    fn reset_clears_all_state() {
        let a = auth0();
        a.handle_request(&post("/api/v2/users", json!({
            "email": "temp@test.com",
            "password": "Pass123"
        })));

        a.reset();

        let snapshot = a.state_snapshot();
        assert_eq!(snapshot["users"], 0);
    }
}
