//! The whole chain against a mock of Microsoft, Xbox Live and the Minecraft services.

use crate::*;
use rusticmc_launcher_core::time;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{body_partial_json, body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const UUID: &str = "069a79f444e94726a5befca90e38aaf5";

struct Env {
    server: MockServer,
    _dir: tempfile::TempDir,
    file: std::path::PathBuf,
    store: Arc<MemoryStore>,
    manager: AccountManager,
}

fn config(base: &str) -> AuthConfig {
    AuthConfig {
        client_id: Some("test-client".into()),
        endpoints: Endpoints::under(base),
        poll: PollTiming {
            min_interval: Duration::from_millis(10),
            slow_down_step: Duration::from_millis(10),
        },
        ..AuthConfig::default()
    }
}

async fn env() -> Env {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("accounts.json");
    let store = Arc::new(MemoryStore::with_item_limit(store::CHUNK));
    let api = AuthApi::new(
        rusticmc_launcher_core::http::client().unwrap(),
        config(&server.uri()),
    );
    let manager = AccountManager::new(file.clone(), store.clone(), api);
    Env {
        server,
        _dir: dir,
        file,
        store,
        manager,
    }
}

async fn mount_device_code(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/ms/devicecode"))
        .and(body_string_contains("client_id=test-client"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user_code": "ABCD-EFGH", "device_code": "dev-123",
            "verification_uri": "https://www.microsoft.com/link",
            "expires_in": 60, "interval": 0, "message": "Go to the link"
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .and(body_string_contains("device_code=dev-123"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(1)
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .and(body_string_contains("device_code=dev-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ms-access", "refresh_token": "ms-refresh", "expires_in": 3600
        })))
        .with_priority(2)
        .mount(server)
        .await;
}

async fn mount_xbox(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/xbl"))
        .and(body_partial_json(
            json!({"Properties": {"RpsTicket": "d=ms-access"}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Token": "xbl-token", "DisplayClaims": {"xui": [{"uhs": "hash1"}]}
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/xsts"))
        .and(body_partial_json(
            json!({"Properties": {"UserTokens": ["xbl-token"]}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Token": "xsts-token", "DisplayClaims": {"xui": [{"uhs": "hash1", "xid": "2535"}]}
        })))
        .mount(server)
        .await;
}

async fn mount_minecraft(server: &MockServer, owns: bool, profile: bool) {
    // `launcher/login` refused: the chain falls back to login_with_xbox.
    Mock::given(method("POST"))
        .and(path("/mc/launcher/login"))
        .respond_with(ResponseTemplate::new(404))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mc/authentication/login_with_xbox"))
        .and(body_partial_json(
            json!({"identityToken": "XBL3.0 x=hash1;xsts-token"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mc-token", "expires_in": 86400
        })))
        .mount(server)
        .await;
    let items = if owns {
        json!([{"name": "product_minecraft"}, {"name": "game_minecraft"}])
    } else {
        json!([])
    };
    Mock::given(method("GET"))
        .and(path("/mc/entitlements/mcstore"))
        .and(header("authorization", "Bearer mc-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": items })))
        .mount(server)
        .await;
    let profile_reply = if profile {
        ResponseTemplate::new(200).set_body_json(json!({
            "id": UUID, "name": "Alex",
            "skins": [{"id": "s1", "url": "https://textures.minecraft.net/texture/abc", "state": "ACTIVE", "variant": "SLIM"}],
            "capes": []
        }))
    } else {
        ResponseTemplate::new(404)
    };
    Mock::given(method("GET"))
        .and(path("/mc/minecraft/profile"))
        .respond_with(profile_reply)
        .mount(server)
        .await;
}

async fn full_chain(server: &MockServer) {
    mount_device_code(server).await;
    mount_xbox(server).await;
    mount_minecraft(server, true, true).await;
}

#[tokio::test]
async fn device_code_sign_in_adds_an_owner() {
    let e = env().await;
    full_chain(&e.server).await;
    let mut events = e.manager.subscribe();
    let flow = e.manager.start_device_code(None).await.unwrap();
    assert_eq!(flow.user_code, "ABCD-EFGH");
    let account = e.manager.complete(&flow.flow_id).await.unwrap();
    assert_eq!(account.uuid, UUID);
    assert_eq!(account.name, "Alex");
    assert!(account.selected);
    assert_eq!(account.state, AccountState::Ready);
    assert_eq!(account.skins[0].variant.as_deref(), Some("SLIM"));
    // Tokens only in the keychain, never in accounts.json.
    let file = std::fs::read_to_string(&e.file).unwrap();
    for token in ["ms-refresh", "mc-token", "ms-access", "xsts-token"] {
        assert!(!file.contains(token), "{token} leaked into accounts.json");
    }
    let raw = e.store.raw_items();
    assert!(raw.contains_key(&format!("minecraft:{UUID}")));
    let mut phases = Vec::new();
    while let Ok(ev) = events.try_recv() {
        phases.push(ev.phase);
    }
    assert!(phases.contains(&AuthPhase::Ownership) && phases.last() == Some(&AuthPhase::Done));
    // Offline play opens with that check.
    assert!(e.manager.offline_access().unwrap().is_allowed());
}

#[tokio::test]
async fn an_account_without_the_game_is_refused_and_not_stored() {
    let e = env().await;
    mount_device_code(&e.server).await;
    mount_xbox(&e.server).await;
    mount_minecraft(&e.server, false, true).await;
    let flow = e.manager.start_device_code(None).await.unwrap();
    let err = e.manager.complete(&flow.flow_id).await.unwrap_err();
    assert!(matches!(err, AuthError::NotOwned), "{err}");
    assert!(e.store.raw_items().is_empty());
    assert!(e.manager.snapshot().unwrap().accounts.is_empty());
    assert!(!e.manager.offline_access().unwrap().is_allowed());
}

#[tokio::test]
async fn an_owner_without_a_profile_is_told_so() {
    let e = env().await;
    mount_device_code(&e.server).await;
    mount_xbox(&e.server).await;
    mount_minecraft(&e.server, true, false).await;
    let flow = e.manager.start_device_code(None).await.unwrap();
    assert!(matches!(
        e.manager.complete(&flow.flow_id).await,
        Err(AuthError::NoProfile)
    ));
}

#[tokio::test]
async fn xsts_errors_are_explained() {
    let e = env().await;
    mount_device_code(&e.server).await;
    Mock::given(method("POST"))
        .and(path("/xbl"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Token": "xbl-token", "DisplayClaims": {"xui": [{"uhs": "h"}]}
        })))
        .mount(&e.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/xsts"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"XErr": 2148916238u64})))
        .mount(&e.server)
        .await;
    let flow = e.manager.start_device_code(None).await.unwrap();
    let err = e.manager.complete(&flow.flow_id).await.unwrap_err();
    assert!(err.to_string().contains("child account"), "{err}");
}

#[tokio::test]
async fn an_unapproved_app_is_reported() {
    let e = env().await;
    mount_device_code(&e.server).await;
    mount_xbox(&e.server).await;
    Mock::given(method("POST"))
        .and(path("/mc/authentication/login_with_xbox"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&e.server)
        .await;
    let flow = e.manager.start_device_code(None).await.unwrap();
    assert!(matches!(
        e.manager.complete(&flow.flow_id).await,
        Err(AuthError::NotApproved)
    ));
}

#[tokio::test]
async fn a_declined_device_code_ends_the_flow() {
    let e = env().await;
    Mock::given(method("POST"))
        .and(path("/ms/devicecode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user_code": "X", "device_code": "d", "verification_uri": "u", "expires_in": 60, "interval": 0
        })))
        .mount(&e.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"error": "authorization_declined"})),
        )
        .mount(&e.server)
        .await;
    let flow = e.manager.start_device_code(None).await.unwrap();
    assert!(matches!(
        e.manager.complete(&flow.flow_id).await,
        Err(AuthError::Declined)
    ));
}

#[tokio::test]
async fn a_flow_can_be_cancelled() {
    let e = env().await;
    Mock::given(method("POST"))
        .and(path("/ms/devicecode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user_code": "X", "device_code": "d", "verification_uri": "u", "expires_in": 60, "interval": 0
        })))
        .mount(&e.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"error": "authorization_pending"})),
        )
        .mount(&e.server)
        .await;
    let manager = Arc::new(e.manager);
    let flow = manager.start_device_code(None).await.unwrap();
    let m2 = manager.clone();
    let id = flow.flow_id.clone();
    let task = tokio::spawn(async move { m2.complete(&id).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    manager.cancel_flow(&flow.flow_id);
    let r = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(r, Err(AuthError::Cancelled)));
}

#[tokio::test]
async fn browser_sign_in_with_pkce() {
    let e = env().await;
    mount_xbox(&e.server).await;
    mount_minecraft(&e.server, true, true).await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth-code-1"))
        .and(body_string_contains("code_verifier="))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ms-access", "refresh_token": "ms-refresh", "expires_in": 3600
        })))
        .expect(1)
        .mount(&e.server)
        .await;
    let manager = Arc::new(e.manager);
    let flow = manager.start_browser(None).await.unwrap();
    let url = url::Url::parse(&flow.authorize_url).unwrap();
    assert!(url.path().ends_with("/ms/authorize"));
    let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["client_id"], "test-client");
    let redirect = q["redirect_uri"].clone();
    assert!(redirect.starts_with("http://localhost:"));
    let state = q["state"].clone();
    let m2 = manager.clone();
    let id = flow.flow_id.clone();
    let task = tokio::spawn(async move { m2.complete(&id).await });
    // The browser comes back.
    let page = rusticmc_launcher_core::http::client()
        .unwrap()
        .get(format!("{redirect}/?code=auth-code-1&state={state}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("Signed in"));
    let account = task.await.unwrap().unwrap();
    assert_eq!(account.uuid, UUID);
}

#[tokio::test]
async fn re_sign_in_refuses_another_account() {
    let e = env().await;
    full_chain(&e.server).await;
    let flow = e
        .manager
        .start_device_code(Some("ffffffffffffffffffffffffffffffff".into()))
        .await
        .unwrap();
    let err = e.manager.complete(&flow.flow_id).await.unwrap_err();
    assert!(matches!(err, AuthError::WrongAccount { .. }));
    assert!(e.store.raw_items().is_empty());
}

#[tokio::test]
async fn a_due_token_is_renewed_before_launch() {
    let e = env().await;
    mount_xbox(&e.server).await;
    mount_minecraft(&e.server, true, true).await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=refresh-old"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ms-access", "refresh_token": "refresh-rotated", "expires_in": 3600
        })))
        .expect(1)
        .mount(&e.server)
        .await;
    // Ten seconds left: due.
    e.manager.seed(UUID, "Alex", time::now() + 10, None);
    let auth = e.manager.launch_session(None).await.unwrap();
    let LaunchAuth::Online(session) = auth else {
        panic!("expected an online session")
    };
    assert_eq!(session.access_token.expose(), "mc-token");
    assert_eq!(session.xuid.as_deref(), Some("2535"));
    let json: serde_json::Value = serde_json::from_str(session.to_json().expose()).unwrap();
    assert_eq!(json["user_type"], "msa");
    let raw = e.store.get(&format!("minecraft:{UUID}")).unwrap().unwrap();
    assert!(raw.contains("refresh-rotated") && !raw.contains("refresh-old"));
    // The launch recorded a fresh ownership answer.
    assert!(e.manager.offline_access().unwrap().is_allowed());
}

#[tokio::test]
async fn a_fresh_token_is_not_renewed() {
    let e = env().await;
    mount_minecraft(&e.server, true, true).await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&e.server)
        .await;
    // `mc-old` is not the token the mock accepts for entitlements... use the real one.
    e.manager.seed(UUID, "Alex", time::now() + 86_000, None);
    e.store
        .set(
            &format!("minecraft:{UUID}"),
            &e.store
                .get(&format!("minecraft:{UUID}"))
                .unwrap()
                .unwrap()
                .replace("mc-old", "mc-token"),
        )
        .unwrap();
    assert!(matches!(
        e.manager.launch_session(Some(UUID)).await.unwrap(),
        LaunchAuth::Online(_)
    ));
}

#[tokio::test]
async fn a_refused_refresh_token_asks_to_sign_in_again() {
    let e = env().await;
    Mock::given(method("POST"))
        .and(path("/ms/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_grant"})))
        .mount(&e.server)
        .await;
    e.manager.seed(UUID, "Alex", 0, None);
    assert!(matches!(
        e.manager.refresh(UUID, true).await,
        Err(AuthError::ReauthRequired)
    ));
    let snap = e.manager.snapshot().unwrap();
    assert_eq!(snap.accounts[0].state, AccountState::SignInAgain);
}

fn unreachable_manager(dir: &std::path::Path) -> (AccountManager, Arc<MemoryStore>) {
    // A port nothing listens on.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let store = Arc::new(MemoryStore::new());
    let api = AuthApi::new(
        rusticmc_launcher_core::http::client().unwrap(),
        config(&format!("http://127.0.0.1:{port}")),
    );
    (
        AccountManager::new(dir.join("accounts.json"), store.clone(), api),
        store,
    )
}

#[tokio::test]
async fn without_network_a_recent_owner_starts_offline() {
    let dir = tempfile::tempdir().unwrap();
    let (manager, _) = unreachable_manager(dir.path());
    manager.seed(
        UUID,
        "Alex",
        0,
        Some(OwnershipRecord {
            owns: true,
            checked_at: time::now() - 86_400,
        }),
    );
    match manager.launch_session(None).await.unwrap() {
        LaunchAuth::Offline { name, session, .. } => {
            assert_eq!(name, "Alex");
            // The game still plays under the official name and UUID, with the last token.
            assert_eq!(session.name, "Alex");
            assert_eq!(session.uuid, UUID);
            assert_eq!(session.access_token.expose(), "mc-old");
        }
        LaunchAuth::Online(_) => panic!("no network: offline"),
    }
}

#[tokio::test]
async fn without_network_an_old_answer_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let (manager, _) = unreachable_manager(dir.path());
    manager.seed(
        UUID,
        "Alex",
        0,
        Some(OwnershipRecord {
            owns: true,
            checked_at: time::now() - 40 * 86_400,
        }),
    );
    let err = manager.launch_session(None).await.unwrap_err();
    assert!(err.is_network());
}

#[tokio::test]
async fn free_name_offline_profiles_of_older_versions_are_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let (manager, _) = unreachable_manager(dir.path());
    manager.seed(UUID, "Alex", 0, None);
    // An older accounts.json with a free-name profile selected.
    let path = dir.path().join("accounts.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    v["offline_profiles"] =
        serde_json::json!([{ "id": "lan", "name": "LanPlayer", "created_at": 1 }]);
    v["selected"] = serde_json::json!("lan");
    std::fs::write(&path, v.to_string()).unwrap();
    // The owner plays, under the official name; the profile can't be selected.
    assert_eq!(
        manager.active().unwrap(),
        ActiveIdentity::Microsoft(UUID.into())
    );
    assert!(manager.select("lan").is_err());
    manager.select(UUID).unwrap();
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("LanPlayer")
    );
}

#[tokio::test]
async fn a_non_owner_answer_blocks_offline() {
    let dir = tempfile::tempdir().unwrap();
    let (manager, _) = unreachable_manager(dir.path());
    manager.seed(
        UUID,
        "Alex",
        0,
        Some(OwnershipRecord {
            owns: false,
            checked_at: time::now(),
        }),
    );
    assert!(manager.launch_session(None).await.is_err());
}

#[tokio::test]
async fn sign_out_removes_the_secrets() {
    let e = env().await;
    e.manager.seed(UUID, "Alex", 0, None);
    e.manager.remove(UUID).await.unwrap();
    assert!(e.store.raw_items().is_empty());
    assert!(e.manager.snapshot().unwrap().accounts.is_empty());
}

#[tokio::test]
async fn no_client_id_means_no_sign_in() {
    let dir = tempfile::tempdir().unwrap();
    let api = AuthApi::new(
        rusticmc_launcher_core::http::client().unwrap(),
        AuthConfig::default(),
    );
    let m = AccountManager::new(dir.path().join("a.json"), Arc::new(MemoryStore::new()), api);
    assert!(matches!(
        m.start_device_code(None).await,
        Err(AuthError::NotConfigured)
    ));
    assert!(!m.snapshot().unwrap().sign_in_available);
}
