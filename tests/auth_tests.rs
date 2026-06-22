use std::fs;

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, StatusCode},
};
use dst_admin_rust::{
    domain::auth::{
        PasswordFile, SessionStore, UserCredentials, is_white_ip, is_whitelisted_path,
        session_cookie_name,
    },
    web::handlers::auth::{
        AuthState, ChangePasswordRequest, UpdateUserInfoRequest, change_password, get_user_info,
        login, login_handler, logout, update_user_info,
    },
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn password_file_parser_accepts_compact_and_spaced_key_value_lines() {
    let dir = tempdir().unwrap();
    let password_path = dir.path().join("password.txt");
    fs::write(
        &password_path,
        "username=admin\npassword = 123456\ndisplayName = Admin User\nphotoURL=https://example.test/avatar.png\n",
    )
    .unwrap();

    let password_file = PasswordFile::read(&password_path).unwrap();

    assert_eq!(password_file.username, "admin");
    assert_eq!(password_file.password, "123456");
    assert_eq!(password_file.display_name, "Admin User");
    assert_eq!(password_file.photo_url, "https://example.test/avatar.png");
}

#[test]
fn password_file_write_preserves_required_keys_and_round_trips() {
    let dir = tempdir().unwrap();
    let password_path = dir.path().join("password.txt");
    let password_file = PasswordFile {
        username: "admin".to_owned(),
        password: "secret-password".to_owned(),
        display_name: "Panel Admin".to_owned(),
        photo_url: "https://example.test/photo.webp".to_owned(),
    };

    password_file.write(&password_path).unwrap();

    let contents = fs::read_to_string(&password_path).unwrap();
    assert!(contents.contains("username = admin"));
    assert!(contents.contains("password = secret-password"));
    assert!(contents.contains("displayName=Panel Admin"));
    assert!(contents.contains("photoURL=https://example.test/photo.webp"));
    assert_eq!(PasswordFile::read(&password_path).unwrap(), password_file);
}

#[test]
fn password_file_public_user_omits_password() {
    let password_file = PasswordFile {
        username: "admin".to_owned(),
        password: "secret-password".to_owned(),
        display_name: "Panel Admin".to_owned(),
        photo_url: "https://example.test/photo.webp".to_owned(),
    };

    let value = serde_json::to_value(password_file.public_user()).unwrap();

    assert_eq!(
        value,
        json!({
            "username": "admin",
            "displayName": "Panel Admin",
            "photoURL": "https://example.test/photo.webp"
        })
    );
    assert!(!value.to_string().contains("secret-password"));
}

#[test]
fn session_store_creates_opaque_sessions_and_remove_invalidates_them() {
    let sessions = SessionStore::new();

    let session_id = sessions.create_session("admin");

    assert_ne!(session_id, "admin");
    assert!(session_id.len() >= 32);
    assert_eq!(sessions.validate(&session_id).as_deref(), Some("admin"));
    assert!(sessions.remove(&session_id));
    assert_eq!(sessions.validate(&session_id), None);
    assert!(!sessions.remove(&session_id));
}

#[test]
fn user_credentials_redacts_sensitive_fields_and_clear_password_empties_password() {
    let mut credentials = UserCredentials {
        username: "admin".to_owned(),
        password: "secret-password".to_owned(),
        session_id: Some("session-secret".to_owned()),
    };

    let serialized = serde_json::to_string(&credentials).unwrap();
    let debug = format!("{credentials:?}");

    assert!(serialized.contains("admin"));
    assert!(!serialized.contains("secret-password"));
    assert!(!serialized.contains("session-secret"));
    assert!(!debug.contains("secret-password"));
    assert!(!debug.contains("session-secret"));

    credentials.clear_password();

    assert_eq!(credentials.password, "");
}

#[test]
fn whitelist_helper_allows_non_api_and_named_compatibility_paths_only() {
    assert!(is_whitelisted_path("/"));
    assert!(is_whitelisted_path("/assets/app.js"));
    assert!(is_whitelisted_path("/api/login"));
    assert!(is_whitelisted_path("/api/logout"));
    assert!(is_whitelisted_path("/ws"));
    assert!(is_whitelisted_path("/api/bootstrap"));
    assert!(is_whitelisted_path("/api/init"));
    assert!(is_whitelisted_path("/api/install/steamcmd"));
    assert!(!is_whitelisted_path("/api/player"));
    assert!(!is_whitelisted_path("/api/login/extra"));
}

#[test]
fn white_admin_ip_supports_exact_ip_and_cidr_entries_with_remote_addr_ports() {
    assert!(is_white_ip("127.0.0.1:49152", Some("10.0.0.1, 127.0.0.1")));
    assert!(is_white_ip(
        "192.168.1.42:8080",
        Some("10.0.0.1, 192.168.1.0/24")
    ));
    assert!(!is_white_ip(
        "192.168.2.42:8080",
        Some("10.0.0.1, 192.168.1.0/24")
    ));
    assert!(!is_white_ip("127.0.0.1:49152", None));
}

#[test]
fn login_success_returns_public_user_payload_and_token_cookie() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(password_path, SessionStore::new(), None);

    let response = login(
        &state,
        None,
        UserCredentials {
            username: "admin".to_owned(),
            password: "123456".to_owned(),
            session_id: None,
        },
    )
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({
            "code": 200,
            "msg": "Login success",
            "data": {
                "username": "admin",
                "displayName": "Admin",
                "photoURL": "https://example.test/avatar.png"
            }
        })
    );
    assert!(
        !serde_json::to_string(&response.body)
            .unwrap()
            .contains("123456")
    );

    let cookie = response.set_cookie.as_deref().unwrap();
    assert!(cookie.starts_with(&format!("{}=", session_cookie_name())));
    assert!(cookie.contains("Max-Age=604800"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Path=/"));

    let session_id = cookie
        .strip_prefix("token=")
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let response_debug = format!("{response:?}");

    assert!(!response_debug.contains(cookie));
    assert!(!response_debug.contains(session_id));
    assert_eq!(
        state.sessions.validate(session_id).as_deref(),
        Some("admin")
    );
}

#[test]
fn login_failure_returns_401_style_body_and_no_cookie() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(password_path, SessionStore::new(), None);

    let response = login(
        &state,
        None,
        UserCredentials {
            username: "admin".to_owned(),
            password: "wrong-password".to_owned(),
            session_id: None,
        },
    )
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({"code": 401, "msg": "User authentication failed", "data": null})
    );
    assert_eq!(response.set_cookie, None);
}

#[test]
fn white_admin_ip_login_bypasses_password_but_still_creates_session_cookie() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(
        password_path,
        SessionStore::new(),
        Some("10.1.0.0/16".to_owned()),
    );

    let response = login(
        &state,
        Some("10.1.2.3:8080"),
        UserCredentials {
            username: "ignored".to_owned(),
            password: "wrong-password".to_owned(),
            session_id: None,
        },
    )
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.data["username"], "admin");
    assert!(response.set_cookie.is_some());
}

#[tokio::test]
async fn login_handler_ignores_spoofed_forwarded_headers_without_trusted_peer_addr() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(
        password_path,
        SessionStore::new(),
        Some("10.1.0.0/16".to_owned()),
    );
    let mut headers = HeaderMap::new();
    headers.insert("x-real-ip", HeaderValue::from_static("10.1.2.3"));
    headers.insert("x-forwarded-for", HeaderValue::from_static("10.1.2.3"));

    let response = login_handler(
        State(state),
        None::<ConnectInfo<std::net::SocketAddr>>,
        headers,
        Json(UserCredentials {
            username: "ignored".to_owned(),
            password: "wrong-password".to_owned(),
            session_id: None,
        }),
    )
    .await
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({"code": 401, "msg": "User authentication failed", "data": null})
    );
    assert_eq!(response.set_cookie, None);
}

#[tokio::test]
async fn login_handler_allows_white_ip_only_from_trusted_peer_addr() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(
        password_path,
        SessionStore::new(),
        Some("10.1.0.0/16".to_owned()),
    );
    let trusted_peer = "10.1.2.3:8080".parse().unwrap();

    let response = login_handler(
        State(state),
        Some(ConnectInfo(trusted_peer)),
        HeaderMap::new(),
        Json(UserCredentials {
            username: "ignored".to_owned(),
            password: "wrong-password".to_owned(),
            session_id: None,
        }),
    )
    .await
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.code, 200);
    assert_eq!(response.body.msg, "Login success");
    assert_eq!(response.body.data["username"], "admin");
    assert!(response.set_cookie.is_some());
}

#[test]
fn logout_removes_session_and_returns_go_success_message() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(password_path, SessionStore::new(), None);
    let session_id = state.sessions.create_session("admin");

    let response = logout(&state, Some(&session_id)).unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({"code": 200, "msg": "Logout success", "data": null})
    );
    assert_eq!(state.sessions.validate(&session_id), None);
    assert!(
        response
            .set_cookie
            .as_deref()
            .unwrap()
            .contains("Max-Age=0")
    );
}

#[test]
fn get_user_info_returns_public_user_payload() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(password_path, SessionStore::new(), None);

    let response = get_user_info(&state).unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({
            "code": 200,
            "msg": "Init user success",
            "data": {
                "username": "admin",
                "displayName": "Admin",
                "photoURL": "https://example.test/avatar.png"
            }
        })
    );
}

#[test]
fn change_password_rewrites_password_file_without_changing_public_user() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(password_path.clone(), SessionStore::new(), None);

    let response = change_password(
        &state,
        ChangePasswordRequest {
            new_password: "new-secret".to_owned(),
        },
    )
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({"code": 200, "msg": "Update user new password success", "data": null})
    );

    let password_file = PasswordFile::read(&password_path).unwrap();
    assert_eq!(password_file.username, "admin");
    assert_eq!(password_file.password, "new-secret");
    assert_eq!(password_file.display_name, "Admin");
    assert_eq!(password_file.photo_url, "https://example.test/avatar.png");
}

#[test]
fn update_user_info_rewrites_all_fields_and_keeps_go_logout_success_message() {
    let dir = tempdir().unwrap();
    let password_path = write_password_file(dir.path());
    let state = AuthState::new(password_path.clone(), SessionStore::new(), None);

    let response = update_user_info(
        &state,
        UpdateUserInfoRequest {
            username: "new-admin".to_owned(),
            display_name: "New Admin".to_owned(),
            photo_url: "https://example.test/new-avatar.png".to_owned(),
            password: "new-secret".to_owned(),
        },
    )
    .unwrap();

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::to_value(&response.body).unwrap(),
        json!({"code": 200, "msg": "Logout success", "data": null})
    );
    assert_eq!(
        PasswordFile::read(&password_path).unwrap(),
        PasswordFile {
            username: "new-admin".to_owned(),
            password: "new-secret".to_owned(),
            display_name: "New Admin".to_owned(),
            photo_url: "https://example.test/new-avatar.png".to_owned(),
        }
    );
}

fn write_password_file(dir: &std::path::Path) -> std::path::PathBuf {
    let password_path = dir.join("password.txt");
    fs::write(
        &password_path,
        "username=admin\npassword=123456\ndisplayName=Admin\nphotoURL=https://example.test/avatar.png\n",
    )
    .unwrap();
    password_path
}
