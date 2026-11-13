//! Compatibility verdicts: how a version runs on the instance's Horizon (`HorizonEnv`), with the
//! reasons. Facts behind each rule (horizon docs, pinned at a84f637d):
//!
//! - Content mods (`.hmod`) load when their manifest `api` is one the build lists (`mod.api`).
//! - Native mods (`.hnmod`) run only in a game *compiled with them*. Rustic Launcher compiles the
//!   Rustic edition client with an instance's native mods on the player's computer (ADR 0157,
//!   `rusticmc-native`), on the edition's own Horizon commit: a mod fits when its `hooks` are at
//!   most the build's `NATIVE_HOOKS_API`; a mod pinned to another commit may not apply there (the
//!   build then leaves it out); the registry's `broken_on` rules it out.
//! - Java mods (Fabric, Forge, NeoForge, Quilt jars) never run: Horizon has no JVM. Their data
//!   and assets can be carried over by `horizon-port-pack` (docs/modpacks.md).
//! - Resource packs always load; outside the build's format they are marked too old or too new.
//! - Horizon shader packs (`.hshader`, WGSL) load when their API line is listed. `shaders/`
//!   (Iris/OptiFine) packs load through Horizon's own GLSL loader (`horizon-shaders`), not every
//!   feature (docs/shaders.md, docs/graphics.md).
//! - Data packs go into a world (`saves/<world>/datapacks/`) or into `new-world-datapacks/`, which
//!   the game copies into each new world; Horizon runs part of a data pack (docs/mod-browser.md).
//! - Rustic Server plugins (`.rplugin`) go only to a server whose engine is `rustic-server`.
use crate::horizon_env::HorizonEnv;
use crate::model::*;
use crate::plugins::ServerInfo;
use crate::sources::modrinth::{JAVA_LOADERS, PLUGIN_LOADERS};

fn c(verdict: Verdict, badge: Badge, reason: impl Into<String>) -> Compatibility {
    Compatibility::new(verdict, badge, reason)
}

/// The verdict for `v` on `env`, and for `target` when given (a data pack needs a world, a
/// plugin a Rustic Server).
pub fn check(v: &ContentVersion, target: Option<&Target>, env: &HorizonEnv) -> Compatibility {
    let facts = v.horizon.clone().unwrap_or_default();
    let java = v.loaders.iter().any(|l| JAVA_LOADERS.contains(&l.as_str()));
    let game_ok = v.game_versions.is_empty() || v.game_versions.iter().any(|g| g == &env.game);
    let mut out = match v.kind {
        ContentKind::Mod if java || v.source == SourceKind::Modrinth => c(
            Verdict::Incompatible,
            Badge::NeedsPort,
            format!(
                "a Java mod ({}): Horizon has no JVM, so its code can't run; its data and \
                 assets can be ported with horizon-port-pack",
                if v.loaders.is_empty() {
                    "jar".to_owned()
                } else {
                    v.loaders.join(", ")
                }
            ),
        ),
        ContentKind::Mod => {
            if !game_ok {
                c(
                    Verdict::Incompatible,
                    Badge::DoesntRun,
                    format!(
                        "made for {}, Horizon plays {}",
                        v.game_versions.join(", "),
                        env.game
                    ),
                )
            } else if env.mod_api.contains(&facts.api) {
                c(
                    Verdict::Compatible,
                    Badge::Works,
                    format!("content mod API {} is loaded by this Horizon", facts.api),
                )
            } else {
                c(
                    Verdict::Incompatible,
                    Badge::DoesntRun,
                    format!(
                        "content mod API {:?}: this Horizon loads {}",
                        facts.api,
                        env.mod_api.join(", ")
                    ),
                )
            }
        }
        ContentKind::NativeMod => native(&facts, env),
        ContentKind::Modpack => match v.source {
            SourceKind::Horizon => {
                if facts.api == env.modpack_format.to_string() || facts.api.is_empty() {
                    c(
                        Verdict::Compatible,
                        Badge::Works,
                        "a Horizon modpack: its mods come from Horizon's registry",
                    )
                } else {
                    c(
                        Verdict::Incompatible,
                        Badge::DoesntRun,
                        format!(
                            ".hpack format {}, this Horizon reads {}",
                            facts.api, env.modpack_format
                        ),
                    )
                }
            }
            _ => c(
                Verdict::Partial,
                Badge::Partly,
                "imported: resource packs, shader packs and data packs come along; Java mods \
                 can't run (their data and assets can be ported with horizon-port-pack)",
            ),
        },
        ContentKind::ResourcePack => {
            if game_ok {
                c(
                    Verdict::Compatible,
                    Badge::Works,
                    format!("made for {}", env.game),
                )
            } else {
                c(
                    Verdict::Partial,
                    Badge::Partly,
                    format!(
                        "made for {}: loads with the game's compatibility warning",
                        v.game_versions.join(", ")
                    ),
                )
            }
        }
        ContentKind::Shader => {
            if env.shader_api.contains(&facts.api) {
                c(
                    Verdict::Compatible,
                    Badge::Works,
                    format!("Horizon shader pack (WGSL), shader API {}", facts.api),
                )
            } else {
                c(
                    Verdict::Incompatible,
                    Badge::DoesntRun,
                    format!(
                        "shader API {:?}: this Horizon reads {}",
                        facts.api,
                        env.shader_api.join(", ")
                    ),
                )
            }
        }
        ContentKind::ShaderPack => c(
            Verdict::Partial,
            Badge::Partly,
            "an Iris/OptiFine (`shaders/`) pack: Horizon runs it with its own GLSL loader, not \
             every feature of those mods; Horizon's own launcher offers only packs reviewed in \
             its registry",
        ),
        ContentKind::DataPack => c(
            Verdict::Partial,
            Badge::Partly,
            "Horizon runs part of a data pack (functions, function tags, predicates and item \
             modifiers per its mod browser; worldgen in part); it goes into one world, or into \
             the packs new worlds start with",
        ),
        ContentKind::Plugin => {
            let bukkit = v
                .loaders
                .iter()
                .any(|l| PLUGIN_LOADERS.contains(&l.as_str()));
            if bukkit {
                c(
                    Verdict::Incompatible,
                    Badge::DoesntRun,
                    "a Bukkit/Paper-family plugin (Java): Rustic Server runs `.rplugin` WebAssembly \
                     plugins (the optional Rustic Bridge is a separate, server-side tool)",
                )
            } else {
                c(Verdict::Compatible, Badge::Works, "a Rustic Server plugin")
            }
        }
    };
    if let Some(t) = target {
        target_rules(v, t, &mut out);
    }
    if !game_ok && v.kind == ContentKind::NativeMod {
        out.reasons
            .push(format!("made for {}", v.game_versions.join(", ")));
    }
    out
}

fn native(facts: &HorizonFacts, env: &HorizonEnv) -> Compatibility {
    let build_note = "native code: Rustic Launcher compiles it into this instance's own client \
                      on your computer, after you accept the trust notice (native mods are not \
                      sandboxed)";
    if !env.native_rebuild {
        return c(
            Verdict::Incompatible,
            Badge::DoesntRun,
            "this Horizon build takes no native mods",
        );
    }
    if let Some(commit) = &env.commit
        && facts.broken_on.iter().any(|b| b == commit)
    {
        return c(
            Verdict::Incompatible,
            Badge::DoesntBuildOnYourHorizon,
            format!("horizon-mixins refused this version on your Horizon ({commit})"),
        );
    }
    match (facts.hooks, env.native_hooks) {
        (Some(need), Some(have)) if have >= need => Compatibility {
            verdict: Verdict::NeedsBuild,
            badge: Badge::BuildsOnYourHorizon,
            reasons: vec![
                format!("needs hook points {need}, your Horizon has {have}"),
                build_note.into(),
            ],
        },
        (Some(need), Some(have)) => c(
            Verdict::Incompatible,
            Badge::NeedsNewerHorizon,
            format!("needs hook points {need}, your Horizon has {have}: update Horizon"),
        ),
        (Some(need), None) => Compatibility {
            verdict: Verdict::NeedsBuild,
            badge: Badge::Partly,
            reasons: vec![
                format!("needs hook points {need}; your Horizon's level is unknown"),
                build_note.into(),
            ],
        },
        (None, _) => {
            // Pinned to one Horizon commit: `native-1/<horizon>/<mixins>`.
            let pin = facts.api.split('/').nth(1).unwrap_or("").to_owned();
            let fits = env
                .commit
                .as_deref()
                .is_some_and(|c| !pin.is_empty() && c == pin);
            Compatibility {
                verdict: Verdict::NeedsBuild,
                badge: if fits {
                    Badge::BuildsOnYourHorizon
                } else {
                    Badge::Partly
                },
                reasons: vec![
                    if fits {
                        "pinned to your Horizon's commit".to_owned()
                    } else {
                        format!(
                            "made for Horizon commit {}: Rustic builds it on its own Horizon, \
                             where it may not apply (the build then leaves it out)",
                            if pin.is_empty() { "?" } else { &pin }
                        )
                    },
                    build_note.into(),
                ],
            }
        }
    }
}

fn target_rules(v: &ContentVersion, t: &Target, out: &mut Compatibility) {
    let wrong = |out: &mut Compatibility, why: &str| {
        *out = c(Verdict::Incompatible, Badge::WrongTarget, why.to_owned());
    };
    match (v.kind, t) {
        (ContentKind::DataPack, Target::World { .. } | Target::NewWorlds { .. }) => {}
        (ContentKind::DataPack, _) => wrong(
            out,
            "a data pack goes into a world (or the packs new worlds start with)",
        ),
        (ContentKind::Plugin, Target::Server { server_dir }) => {
            let info = ServerInfo::detect(server_dir);
            if !info.rustic {
                wrong(
                    out,
                    "this server runs vanilla Horizon: plugins need Rustic Server (engine \
                     `rustic-server`)",
                );
            }
        }
        (ContentKind::Plugin, _) => wrong(out, "a plugin goes into a Rustic Server server"),
        (_, Target::Server { .. })
            if matches!(
                v.kind,
                ContentKind::ResourcePack | ContentKind::Shader | ContentKind::ShaderPack
            ) =>
        {
            wrong(out, "a client-side pack goes into an instance")
        }
        (_, Target::World { .. } | Target::NewWorlds { .. }) => {
            wrong(out, "only data packs go into worlds")
        }
        _ => {}
    }
}

/// A downloaded resource or data pack's `pack.mcmeta` against the build's format: inside, it
/// works; outside, the game marks it too old or too new (it still loads).
pub fn check_pack_file(
    path: &std::path::Path,
    kind: ContentKind,
    env: &HorizonEnv,
) -> crate::Result<Compatibility> {
    let mcmeta = crate::inspect::read_entry(path, "pack.mcmeta")?.ok_or_else(|| {
        crate::ContentError::Refused("no pack.mcmeta at the root of the zip".into())
    })?;
    let want = if kind == ContentKind::DataPack {
        env.datapack_format.0
    } else {
        env.resourcepack_format.0
    };
    Ok(match crate::inspect::pack_formats(&mcmeta) {
        Some((lo, hi)) if lo <= want && want <= hi => c(
            Verdict::Compatible,
            Badge::Works,
            format!("pack format {lo}..={hi} includes {want}"),
        ),
        Some((lo, hi)) => c(
            Verdict::Partial,
            Badge::Partly,
            format!(
                "pack format {lo}..={hi}, this Horizon's is {want}: it loads marked too {}",
                if hi < want { "old" } else { "new" }
            ),
        ),
        None => c(
            Verdict::Partial,
            Badge::Partly,
            "pack.mcmeta names no format",
        ),
    })
}
