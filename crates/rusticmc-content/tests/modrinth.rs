//! Modrinth API v2 against wiremock fixtures: search facets, the User-Agent, versions with
//! hashes and dependencies, rate limits.
use rusticmc_content::model::*;
use rusticmc_content::sources::modrinth::*;
use std::time::Duration;
use wiremock::matchers::{header_regex, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Modrinth {
    Modrinth::new(ModrinthConfig {
        base_url: format!("{}/v2", server.uri()),
        contact: "ops@rustic.example".into(),
        max_retries: 3,
        base_delay: Duration::from_millis(5),
        max_delay: Duration::from_millis(50),
    })
    .unwrap()
}

const SEARCH: &str = r#"{ "hits": [
  { "project_id": "AANobbMI", "slug": "sodium", "title": "Sodium", "description": "Rendering engine",
    "categories": ["fabric", "optimization"], "display_categories": ["fabric", "optimization", "neoforge"],
    "project_type": "mod", "downloads": 100, "icon_url": "https://cdn.modrinth.com/i.png", "author": "jellysquid3",
    "versions": ["26.3"], "license": "LGPL-3.0-only" },
  { "project_id": "RP1", "slug": "faithful", "title": "Faithful 32x", "description": "Textures",
    "categories": ["minecraft"], "project_type": "resourcepack", "downloads": 50, "author": "f", "versions": ["26.3"] }
], "offset": 0, "limit": 20, "total_hits": 2 }"#;

#[tokio::test]
async fn search_sends_facets_and_an_identifying_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/search"))
        .and(query_param("query", "sodium"))
        .and(query_param(
            "facets",
            r#"[["project_type:mod"],["versions:26.3"],["categories:optimization"]]"#,
        ))
        .and(header_regex(
            "user-agent",
            r"^rustic-launcher/\d+\.\d+\.\d+ \(ops@rustic\.example\)$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SEARCH))
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server)
        .search(&SearchQuery {
            query: "sodium".into(),
            kind: Some(ContentKind::Mod),
            facets: vec![vec!["categories:optimization".into()]],
            game_version: Some("26.3".into()),
            ..SearchQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.hits[0].kind, ContentKind::Mod);
    assert_eq!(page.hits[0].loaders, vec!["fabric"]);
    assert_eq!(page.hits[0].tags, vec!["neoforge"]);
    assert_eq!(page.hits[1].kind, ContentKind::ResourcePack);
}

const VERSIONS: &str = r#"[
  { "id": "old", "project_id": "RP1", "name": "1.0", "version_number": "1.0.0", "game_versions": ["26.3"],
    "loaders": ["minecraft"], "date_published": "2026-01-01T00:00:00Z", "dependencies": [],
    "files": [{ "url": "https://cdn.modrinth.com/data/RP1/versions/old/f.zip", "filename": "f.zip", "primary": true, "size": 3,
                "hashes": { "sha1": "aaa", "sha512": "bbb" } }] },
  { "id": "new", "project_id": "RP1", "name": "2.0", "version_number": "2.0.0", "game_versions": ["26.3"],
    "loaders": ["minecraft"], "date_published": "2026-10-01T00:00:00Z",
    "dependencies": [{ "project_id": "LIB", "version_id": null, "dependency_type": "required" },
                     { "project_id": "BAD", "dependency_type": "incompatible" }],
    "files": [{ "url": "https://cdn.modrinth.com/data/RP1/versions/new/f.zip", "filename": "f.zip", "primary": true, "size": 4,
                "hashes": { "sha1": "ccc", "sha512": "ddd" } }] }
]"#;

#[tokio::test]
async fn versions_carry_files_hashes_and_dependencies_newest_first() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/project/RP1/version"))
        .and(query_param("game_versions", r#"["26.3"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_string(VERSIONS))
        .mount(&server)
        .await;
    let v = client(&server)
        .versions("RP1", &[], &["26.3"], Some("resourcepack"))
        .await
        .unwrap();
    assert_eq!(v[0].id, "new");
    assert_eq!(v[0].kind, ContentKind::ResourcePack);
    assert_eq!(v[0].files[0].hashes.sha512.as_deref(), Some("ddd"));
    assert_eq!(v[0].dependencies[0].kind, DependencyKind::Required);
    assert_eq!(v[0].dependencies[1].kind, DependencyKind::Incompatible);
}

#[tokio::test]
async fn rate_limit_is_retried_after_the_reset() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/version/abc"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-limit", "300")
                .insert_header("x-ratelimit-remaining", "0")
                .insert_header("x-ratelimit-reset", "1"),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/version/abc"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ratelimit-remaining", "299")
                .set_body_string(
                    r#"{ "id": "abc", "project_id": "P", "version_number": "1.0.0", "files": [] }"#,
                ),
        )
        .mount(&server)
        .await;
    let started = std::time::Instant::now();
    let v = client(&server).version("abc").await.unwrap();
    assert_eq!(v.id, "abc");
    // Two waits, each the reset capped at max_delay (50 ms).
    assert!(started.elapsed() >= Duration::from_millis(90));
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn persistent_rate_limit_gives_up() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("x-ratelimit-reset", "0"))
        .mount(&server)
        .await;
    let err = client(&server).version("abc").await.unwrap_err();
    assert!(matches!(
        err,
        rusticmc_content::ContentError::RateLimited(_)
    ));
    // The first try and three retries.
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn server_errors_back_off_then_succeed_and_404_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/project/x"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/project/x"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{ "id": "x", "slug": "x", "title": "X" }"#),
        )
        .mount(&server)
        .await;
    assert_eq!(client(&server).project("x").await.unwrap().title, "X");
    assert!(matches!(
        client(&server).project("missing").await.unwrap_err(),
        rusticmc_content::ContentError::NotFound(_)
    ));
}

#[test]
fn kinds_from_project_types_and_loaders() {
    assert_eq!(kind_of("mod", &["datapack".into()]), ContentKind::DataPack);
    assert_eq!(kind_of("mod", &["fabric".into()]), ContentKind::Mod);
    assert_eq!(kind_of("shader", &["iris".into()]), ContentKind::ShaderPack);
    // `/version` objects have no project type (QA regression: resource packs were refused as
    // Java mods with loader "minecraft").
    assert_eq!(
        kind_of("mod", &["minecraft".into()]),
        ContentKind::ResourcePack
    );
    assert_eq!(
        kind_of("mod", &["iris".into(), "optifine".into()]),
        ContentKind::ShaderPack
    );
    assert_eq!(
        kind_of("mod", &["fabric".into(), "minecraft".into()]),
        ContentKind::Mod
    );
    assert!(user_agent("me@x").starts_with("rustic-launcher/"));
}
