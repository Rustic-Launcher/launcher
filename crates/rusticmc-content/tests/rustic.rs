//! The Rustic source against the API's real wire format: every body below is serialized from
//! `rusticmc_api_client::types` (the structs the API itself answers with, ADR 0155), served by
//! wiremock, and read back by `HttpRusticRegistry`.
use chrono::{TimeZone, Utc};
use rusticmc_api_client::types as wire;
use rusticmc_content::compat;
use rusticmc_content::horizon_env::HorizonEnv;
use rusticmc_content::model::*;
use rusticmc_content::sources::rustic::*;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn when(day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, 10, 0, 0).unwrap()
}

fn project(id: &str, kind: wire::ContentKind) -> wire::ContentProject {
    wire::ContentProject {
        source: "rustic".into(),
        id: id.into(),
        slug: id.into(),
        kind,
        name: "Sky Islands".into(),
        summary: "Floating islands for new worlds.".into(),
        authors: vec!["lea".into()],
        icon_url: Some(format!(
            "https://cdn.rustic-launcher.com/marketplace/media/{id}/icon.png"
        )),
        downloads: 1234,
        categories: vec!["worldgen".into()],
        tags: vec!["islands".into()],
        license: Some("MIT".into()),
        loaders: vec![],
        game_versions: vec!["26.3".into()],
        fair_play: wire::ContentFairPlay {
            status: wire::FairPlayStatus::Clean,
            reason: None,
        },
        description: None,
        gallery: vec![wire::ContentImage {
            url: format!("https://cdn.rustic-launcher.com/marketplace/media/{id}/a.png"),
            width: 1920,
            height: 1080,
        }],
        links: wire::ContentLinks {
            source: Some("https://example.org/src".into()),
            ..Default::default()
        },
        owner: Some(wire::PublicUser {
            id: uuid::Uuid::new_v4(),
            username: "lea_builds".into(),
            display_name: "Léa".into(),
        }),
        visibility: wire::ContentVisibility::Public,
        verified: true,
        price: wire::ContentPrice {
            amount_cents: 0,
            currency: wire::Currency::Eur,
        },
        created_at: Some(when(1)),
        updated_at: Some(when(12)),
        latest_version: Some("1.2.0".into()),
        size: Some(52_000),
        review: wire::ContentReviewStatus::NotRequired,
        marketplace: true,
        moderation: None,
    }
}

fn version(project: &str, v: &str, kind: wire::ContentKind, api: &str) -> wire::ContentVersion {
    let id = format!("{project}@{v}");
    wire::ContentVersion {
        source: "rustic".into(),
        project_id: project.into(),
        id: id.clone(),
        name: format!("Sky Islands {v}"),
        version: v.into(),
        kind,
        game_versions: vec!["26.3".into()],
        loaders: vec![],
        files: vec![wire::ContentFile {
            url: format!(
                "https://api.rustic-launcher.com/v1/content/files/{project}%40{v}/{project}-{v}.hmod"
            ),
            filename: format!("{project}-{v}.hmod"),
            size: Some(52_000),
            hashes: wire::ContentHashes {
                sha1: None,
                sha256: Some("a".repeat(64)),
                sha512: Some("b".repeat(128)),
            },
            primary: true,
            signature: None,
        }],
        dependencies: vec![wire::ContentDependency {
            project_id: Some("core_lib".into()),
            version_id: None,
            version_req: None,
            sha256: None,
            kind: wire::ContentDependencyKind::Required,
        }],
        published: when(12).to_rfc3339(),
        horizon: Some(wire::ContentHorizonFacts {
            api: api.into(),
            hooks: None,
            side: Some("client".into()),
            capabilities: vec![],
            broken_on: vec![],
            key: None,
        }),
        changelog: "Taller islands.".into(),
        channel: wire::ContentChannel::Beta,
        yanked: false,
        yanked_reason: None,
        downloads: 99,
        review: wire::ContentReviewStatus::NotRequired,
        review_note: None,
    }
}

fn json<T: serde::Serialize>(v: &T) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::to_value(v).unwrap())
}

#[tokio::test]
async fn search_sends_the_marketplace_filters_and_reads_the_cards() {
    let server = MockServer::start().await;
    let mut paid = project("cozy_pack", wire::ContentKind::ResourcePack);
    paid.price.amount_cents = 499;
    paid.verified = false;
    paid.owner = None;
    let body = wire::ContentSearchResponse {
        hits: vec![project("sky_islands", wire::ContentKind::Mod), paid],
        has_more: true,
        total: 31,
    };
    Mock::given(method("GET"))
        .and(path("/v1/content/search"))
        .and(query_param("q", "sky"))
        .and(query_param("kind", "mod"))
        .and(query_param("category", "worldgen"))
        .and(query_param("sort", "newest"))
        .and(query_param("price", "free"))
        .and(query_param("verified", "true"))
        .and(query_param("offset", "12"))
        .and(query_param("limit", "12"))
        .respond_with(json(&body))
        .expect(1)
        .mount(&server)
        .await;
    let page = HttpRusticRegistry::new(server.uri())
        .unwrap()
        .search(&RusticSearch {
            query: "sky".into(),
            kind: Some(ContentKind::Mod),
            category: Some("worldgen".into()),
            sort: RusticSort::Newest,
            price: Some(PriceFilter::Free),
            verified: true,
            offset: 12,
            limit: 12,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(page.has_more);
    assert_eq!(page.total, 31);
    assert_eq!(page.hits.len(), 2);
    let sky = &page.hits[0];
    assert_eq!(sky.summary.source, SourceKind::Rustic);
    assert_eq!(sky.summary.kind, ContentKind::Mod);
    assert_eq!(sky.summary.fair_play, FairPlay::Clean);
    assert_eq!(sky.summary.downloads, 1234);
    assert!(sky.verified && sky.marketplace && !sky.is_paid());
    assert_eq!(sky.author(), "Léa");
    assert_eq!(sky.gallery[0].width, 1920);
    assert_eq!(sky.size, Some(52_000));
    assert_eq!(sky.latest_version.as_deref(), Some("1.2.0"));
    assert!(sky.updated_at.as_deref().unwrap().starts_with("2026-11-12"));
    let cozy = &page.hits[1];
    assert_eq!(cozy.summary.kind, ContentKind::ResourcePack);
    assert!(cozy.is_paid());
    assert_eq!(cozy.price.as_ref().unwrap().currency, "EUR");
    // No owner: the authors list credits it.
    assert_eq!(cozy.author(), "lea");
}

#[tokio::test]
async fn an_api_older_than_the_marketplace_still_pages() {
    // Production before the marketplace deploy answers `{"hits": [...]}` only, with the pre-0155
    // project fields.
    let server = MockServer::start().await;
    let old = serde_json::json!({ "hits": [{
        "source": "rustic", "id": "claims", "slug": "claims", "kind": "plugin", "name": "Claims",
        "summary": "Land claims", "authors": ["Rustic"], "icon_url": null, "downloads": 5,
        "categories": [], "tags": [], "license": null, "loaders": [], "game_versions": [],
        "fair_play": { "status": "flagged" }
    }] });
    Mock::given(method("GET"))
        .and(path("/v1/content/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(old))
        .mount(&server)
        .await;
    let reg = HttpRusticRegistry::new(server.uri()).unwrap();
    let page = reg
        .search(&RusticSearch {
            limit: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    // A full page may have more.
    assert!(page.has_more);
    assert_eq!(page.total, 2);
    let p = &page.hits[0];
    assert!(!p.verified && !p.is_paid() && p.gallery.is_empty());
    assert!(matches!(p.summary.fair_play, FairPlay::Flagged { .. }));
    let page = reg
        .search(&RusticSearch {
            limit: 20,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!page.has_more);
    assert_eq!(page.total, 1);
}

#[tokio::test]
async fn project_and_versions_carry_the_facts_verdicts_need() {
    let server = MockServer::start().await;
    let mut detail = project("sky_islands", wire::ContentKind::Mod);
    detail.description = Some("# Sky Islands\n\n- **Floating** islands".into());
    Mock::given(method("GET"))
        .and(path("/v1/content/projects/sky_islands"))
        .respond_with(json(&detail))
        .mount(&server)
        .await;
    let mut yanked = version("sky_islands", "1.1.9", wire::ContentKind::Mod, "0.2");
    yanked.yanked = true;
    let versions = vec![
        version("sky_islands", "1.2.0", wire::ContentKind::Mod, "0.2"),
        yanked,
        version("sky_islands", "1.0.0", wire::ContentKind::Mod, "9.9"),
    ];
    Mock::given(method("GET"))
        .and(path("/v1/content/projects/sky_islands/versions"))
        .respond_with(json(&versions))
        .mount(&server)
        .await;
    let reg = HttpRusticRegistry::new(format!("{}/", server.uri())).unwrap();

    let p = reg.project("sky_islands").await.unwrap();
    assert_eq!(
        p.description.as_deref(),
        Some("# Sky Islands\n\n- **Floating** islands")
    );
    assert_eq!(p.links.source.as_deref(), Some("https://example.org/src"));

    let v = reg.versions("sky_islands").await.unwrap();
    assert_eq!(v.len(), 2, "yanked versions are dropped");
    let newest = &v[0];
    assert_eq!(newest.version.id, "sky_islands@1.2.0");
    assert_eq!(newest.version.source, SourceKind::Rustic);
    assert_eq!(newest.channel, Channel::Beta);
    assert_eq!(newest.changelog, "Taller islands.");
    assert_eq!(newest.downloads, 99);
    let f = newest.version.primary_file().unwrap();
    assert_eq!(f.size, Some(52_000));
    assert_eq!(f.hashes.sha512.as_deref(), Some("b".repeat(128).as_str()));
    assert!(
        rusticmc_content::install::check_origin(SourceKind::Rustic, &f.url, &f.filename).is_ok()
    );
    assert_eq!(
        newest.version.dependencies[0].project_id.as_deref(),
        Some("core_lib")
    );

    // `horizon` is HorizonFacts: the verdicts work as for Horizon's registry.
    let env = HorizonEnv::default();
    assert_eq!(
        compat::check(&newest.version, None, &env).verdict,
        Verdict::Compatible
    );
    assert_eq!(
        compat::check(&v[1].version, None, &env).verdict,
        Verdict::Incompatible
    );
}

#[tokio::test]
async fn native_mods_need_a_build() {
    let server = MockServer::start().await;
    let mut v = version(
        "fast_paths",
        "0.3.0",
        wire::ContentKind::NativeMod,
        "native-1/abc/def",
    );
    v.horizon.as_mut().unwrap().hooks = Some(1);
    Mock::given(method("GET"))
        .and(path("/v1/content/projects/fast_paths/versions"))
        .respond_with(json(&vec![v]))
        .mount(&server)
        .await;
    let got = HttpRusticRegistry::new(server.uri())
        .unwrap()
        .versions("fast_paths")
        .await
        .unwrap();
    assert_eq!(got[0].version.kind, ContentKind::NativeMod);
    let c = compat::check(&got[0].version, None, &HorizonEnv::default());
    assert_eq!(c.verdict, Verdict::NeedsBuild);
}

#[tokio::test]
async fn hidden_projects_are_not_found_and_bad_ids_never_reach_the_api() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/content/projects/private_one"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": { "code": "not_found", "message": "project not found" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let reg = HttpRusticRegistry::new(server.uri()).unwrap();
    assert!(matches!(
        reg.project("private_one").await,
        Err(rusticmc_content::ContentError::NotFound(_))
    ));
    for bad in ["../admin", "a/b", "Sky", "", "x?y=1"] {
        assert!(matches!(
            reg.project(bad).await,
            Err(rusticmc_content::ContentError::NotFound(_))
        ));
        assert!(matches!(
            reg.versions(bad).await,
            Err(rusticmc_content::ContentError::NotFound(_))
        ));
    }
}

#[tokio::test]
async fn server_errors_are_reported() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/content/search"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .mount(&server)
        .await;
    let err = HttpRusticRegistry::new(server.uri())
        .unwrap()
        .search(&RusticSearch::default())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        rusticmc_content::ContentError::Http { status: 429, .. }
    ));
}

/// Against a running API: `RUSTIC_API=http://localhost:8080 cargo test -p rusticmc-content
/// --test rustic -- --ignored live`.
#[tokio::test]
#[ignore = "needs a running Rustic API (RUSTIC_API)"]
async fn live_marketplace() {
    let base = std::env::var("RUSTIC_API").unwrap_or_else(|_| "http://localhost:8080".into());
    let reg = HttpRusticRegistry::new(base).unwrap();
    let page = reg
        .search(&RusticSearch {
            limit: 5,
            ..Default::default()
        })
        .await
        .unwrap();
    println!(
        "{} hit(s), total {}, more {}",
        page.hits.len(),
        page.total,
        page.has_more
    );
    let env = HorizonEnv::default();
    for p in &page.hits {
        let v = reg.versions(&p.summary.id).await.unwrap();
        let verdict = v
            .first()
            .map(|x| compat::check(&x.version, None, &env).verdict);
        println!(
            "{} {:?} by {} verified={} paid={} versions={} verdict={verdict:?}",
            p.summary.id,
            p.summary.kind,
            p.author(),
            p.verified,
            p.is_paid(),
            v.len()
        );
        let detail = reg.project(&p.summary.id).await.unwrap();
        assert_eq!(detail.summary.id, p.summary.id);
    }
}
