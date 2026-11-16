//! The command that starts a version: JVM arguments (the file's `arguments.jvm`, or the official
//! launcher's defaults for old files), memory, the logging setup, the main class, then the game
//! arguments (`arguments.game` or legacy `minecraftArguments`), each `${...}` filled in; the
//! classpath, the natives folder (old natives unpacked) and the readable asset copies old
//! versions read.
//!
//! The account's token is a game argument (Java Edition takes it no other way). On Java 9+ every
//! argument goes into a Java argument file (`java @file`), readable by this user only and deleted
//! once the game has started, so the token is never on a command line other programs can read; on
//! Java 8 (old versions) it has to be on the command line, as with every launcher. The token never
//! goes to the logs ([`LaunchCommand::display`] masks it).

use crate::assets::AssetIndex;
use crate::install::Installed;
use crate::platform::{Arch, Os, Platform};
use crate::rules::{Features, allowed};
use crate::version::Argument;
use crate::{JavaError, Result};
use rusticmc_launcher_core::Secret;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Who plays: the Microsoft account's game profile and token.
#[derive(Clone, Debug)]
pub struct Auth {
    pub name: String,
    /// Undashed UUID.
    pub uuid: String,
    pub access_token: Secret,
    pub xuid: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LaunchOptions {
    pub game_dir: PathBuf,
    pub java: PathBuf,
    /// The Java major (argument file from 9), when known.
    pub java_major: Option<u32>,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: bool,
    /// The profile's own JVM arguments; they replace the version's suggested defaults.
    pub jvm_args: Vec<String>,
    pub launcher_name: String,
    pub launcher_version: String,
    /// Join this server on start (`host[:port]`), on versions with quick play.
    pub quick_play_server: Option<String>,
}

/// A command ready to spawn.
#[derive(Clone)]
pub struct LaunchCommand {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Values masked in [`LaunchCommand::display`].
    secrets: Vec<String>,
    /// Put the arguments in an argument file (Java 9+).
    pub use_argfile: bool,
}

impl std::fmt::Debug for LaunchCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display())
    }
}

impl LaunchCommand {
    /// The command line with secrets masked (logs, the UI's "copy command").
    pub fn display(&self) -> String {
        let mut out = vec![self.java.display().to_string()];
        for a in &self.args {
            let mut a = a.clone();
            for s in &self.secrets {
                if !s.is_empty() {
                    a = a.replace(s.as_str(), "********");
                }
            }
            out.push(if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a
            });
        }
        out.join(" ")
    }

    /// The argument file's text: one quoted argument per line (`\` and `"` escaped).
    pub fn argfile_text(&self) -> String {
        let mut s = String::new();
        for a in &self.args {
            s.push('"');
            for c in a.chars() {
                match c {
                    '\\' => s.push_str("\\\\"),
                    '"' => s.push_str("\\\""),
                    '\n' => s.push_str("\\n"),
                    '\r' => s.push_str("\\r"),
                    c => s.push(c),
                }
            }
            s.push_str("\"\n");
        }
        s
    }
}

/// The values of `${...}` placeholders.
pub type Vars = HashMap<&'static str, String>;

/// Replaces every `${key}` the map knows in `s` (unknown ones stay as they are).
pub fn substitute(s: &str, vars: &Vars) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match vars.get(key) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str("${");
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// The values of a list of arguments allowed for this platform and these features.
pub fn evaluate(args: &[Argument], platform: &Platform, features: &Features) -> Vec<String> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Argument::Plain(s) => out.push(s.clone()),
            Argument::Ruled { rules, value } => {
                if allowed(rules, platform, features) {
                    out.extend(value.values());
                }
            }
        }
    }
    out
}

/// The JVM arguments of old version files (the official launcher's built-in list).
fn legacy_jvm(platform: &Platform) -> Vec<String> {
    let mut v = Vec::new();
    if platform.os == Os::Osx {
        v.push("-XstartOnFirstThread".to_owned());
    }
    if platform.os == Os::Windows {
        v.push(
            "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"
                .to_owned(),
        );
    }
    if platform.arch == Arch::X86 {
        v.push("-Xss1M".to_owned());
    }
    v.extend(
        [
            "-Djava.library.path=${natives_directory}",
            "-Dminecraft.launcher.brand=${launcher_name}",
            "-Dminecraft.launcher.version=${launcher_version}",
            "-cp",
            "${classpath}",
        ]
        .map(str::to_owned),
    );
    v
}

/// The features this launch turns on.
pub fn features_for(opts: &LaunchOptions) -> Features {
    let mut f = Features::none();
    if opts.resolution.is_some() {
        f = f.with("has_custom_resolution");
    }
    if opts.quick_play_server.is_some() {
        f = f.with("is_quick_play_multiplayer");
    }
    f
}

/// Builds the command for an installed version. `root` is the game folder of [`Installed`],
/// `natives_dir` the folder [`prepare`] filled.
pub fn build(
    installed: &Installed,
    root: &Path,
    natives_dir: &Path,
    auth: &Auth,
    opts: &LaunchOptions,
    platform: &Platform,
) -> Result<LaunchCommand> {
    let v = &installed.version;
    let features = features_for(opts);
    let sep = platform.classpath_separator();
    let mut classpath: Vec<String> = installed
        .libraries
        .classpath
        .iter()
        .map(|p| root.join(p).display().to_string())
        .collect();
    if let Some(jar) = &installed.client_jar {
        classpath.push(root.join(jar).display().to_string());
    }
    let assets_root = root.join("assets");
    let index_id = installed.asset_index.clone().unwrap_or_default();
    let game_assets = match &installed.asset_index {
        Some(id) => {
            let index = read_index(root, id).unwrap_or_default();
            index
                .copies_dir(root, id, &opts.game_dir)
                .unwrap_or_else(|| assets_root.clone())
        }
        None => assets_root.clone(),
    };
    let token = auth.access_token.expose().to_owned();
    let mut vars: Vars = HashMap::new();
    vars.insert("auth_player_name", auth.name.clone());
    vars.insert("version_name", v.id.clone());
    vars.insert("game_directory", opts.game_dir.display().to_string());
    vars.insert("assets_root", assets_root.display().to_string());
    vars.insert("game_assets", game_assets.display().to_string());
    vars.insert("assets_index_name", index_id);
    vars.insert("auth_uuid", auth.uuid.clone());
    vars.insert("auth_access_token", token.clone());
    vars.insert("auth_session", format!("token:{token}:{}", auth.uuid));
    vars.insert("auth_xuid", auth.xuid.clone().unwrap_or_else(|| "0".into()));
    vars.insert("clientid", uuid::Uuid::new_v4().simple().to_string());
    vars.insert("user_type", "msa".into());
    vars.insert("user_properties", "{}".into());
    vars.insert(
        "version_type",
        v.kind.clone().unwrap_or_else(|| "release".into()),
    );
    vars.insert("natives_directory", natives_dir.display().to_string());
    vars.insert("launcher_name", opts.launcher_name.clone());
    vars.insert("launcher_version", opts.launcher_version.clone());
    vars.insert("classpath", classpath.join(sep));
    vars.insert("classpath_separator", sep.to_owned());
    vars.insert(
        "library_directory",
        root.join("libraries").display().to_string(),
    );
    if let Some((w, h)) = opts.resolution {
        vars.insert("resolution_width", w.to_string());
        vars.insert("resolution_height", h.to_string());
    }
    if let Some(server) = &opts.quick_play_server {
        vars.insert("quickPlayMultiplayer", server.clone());
    }

    let mut args: Vec<String> = vec![
        format!("-Xms{}M", opts.memory_min_mb.min(opts.memory_max_mb)),
        format!("-Xmx{}M", opts.memory_max_mb),
    ];
    if opts.jvm_args.is_empty() {
        if let Some(a) = &v.arguments {
            args.extend(
                evaluate(&a.default_user_jvm, platform, &features)
                    .into_iter()
                    .filter(|s| !s.starts_with("-Xms") && !s.starts_with("-Xmx")),
            );
        }
    } else {
        args.extend(opts.jvm_args.iter().cloned());
    }
    let jvm = match &v.arguments {
        Some(a) if !a.jvm.is_empty() => evaluate(&a.jvm, platform, &features),
        _ => legacy_jvm(platform),
    };
    args.extend(jvm.iter().map(|s| substitute(s, &vars)));
    if let (Some(log), Some(config)) = (
        v.logging.as_ref().and_then(|l| l.client.as_ref()),
        &installed.logging_config,
    ) {
        let mut lv = vars.clone();
        lv.insert("path", root.join(config).display().to_string());
        args.push(substitute(&log.argument, &lv));
    }
    let main = v
        .main_class
        .clone()
        .ok_or_else(|| JavaError::bad("the version has no main class"))?;
    args.push(main);
    let modern_game = v.arguments.as_ref().is_some_and(|a| !a.game.is_empty());
    if modern_game {
        let game = evaluate(
            &v.arguments.as_ref().expect("checked").game,
            platform,
            &features,
        );
        args.extend(game.iter().map(|s| substitute(s, &vars)));
    } else if let Some(legacy) = &v.minecraft_arguments {
        args.extend(legacy.split_whitespace().map(|s| substitute(s, &vars)));
        if let Some((w, h)) = opts.resolution {
            args.extend([
                "--width".into(),
                w.to_string(),
                "--height".into(),
                h.to_string(),
            ]);
        }
    }
    if opts.fullscreen {
        args.push("--fullscreen".into());
    }
    Ok(LaunchCommand {
        java: opts.java.clone(),
        args,
        cwd: opts.game_dir.clone(),
        secrets: vec![token],
        use_argfile: opts.java_major.is_some_and(|m| m >= 9),
    })
}

fn read_index(root: &Path, id: &str) -> Option<AssetIndex> {
    let text =
        std::fs::read_to_string(root.join("assets/indexes").join(format!("{id}.json"))).ok()?;
    AssetIndex::parse(&text).ok()
}

/// Before a launch: every classpath file is there, the natives folder is fresh with the old-style
/// natives unpacked, and versions before 1.6 have their resources copied into the game folder.
pub fn prepare(
    installed: &Installed,
    root: &Path,
    natives_dir: &Path,
    game_dir: &Path,
) -> Result<()> {
    for p in installed
        .libraries
        .classpath
        .iter()
        .chain(installed.client_jar.iter())
    {
        if !root.join(p).is_file() {
            return Err(JavaError::MissingFile(root.join(p)));
        }
    }
    std::fs::create_dir_all(game_dir).map_err(|e| JavaError::io(game_dir, e))?;
    let _ = std::fs::remove_dir_all(natives_dir);
    std::fs::create_dir_all(natives_dir).map_err(|e| JavaError::io(natives_dir, e))?;
    for native in &installed.libraries.natives {
        extract_natives(&root.join(&native.path), natives_dir, &native.exclude)?;
    }
    if let Some(id) = &installed.asset_index
        && let Some(index) = read_index(root, id)
        && index.map_to_resources
    {
        let dir = game_dir.join("resources");
        index.place_copies(root, &dir)?;
    }
    Ok(())
}

/// Unpacks a natives jar's files (not `exclude`d, never outside `dir`).
pub fn extract_natives(jar: &Path, dir: &Path, exclude: &[String]) -> Result<()> {
    let file = std::fs::File::open(jar).map_err(|e| JavaError::io(jar, e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| JavaError::bad(format!("{}: {e}", jar.display())))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| JavaError::bad(format!("{}: {e}", jar.display())))?;
        let name = entry.name().to_owned();
        if entry.is_dir() || exclude.iter().any(|x| name.starts_with(x.as_str())) {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let to = dir.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| JavaError::io(parent, e))?;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| JavaError::io(&to, e))?;
        std::fs::write(&to, bytes).map_err(|e| JavaError::io(&to, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libraries::{LibraryPlan, NativeJar};
    use crate::version::VersionJson;
    use serde_json::json;

    fn win() -> Platform {
        Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
            os_version: Some("10.0.26200".into()),
        }
    }

    fn auth() -> Auth {
        Auth {
            name: "Alex".into(),
            uuid: "069a79f444e94726a5befca90e38aaf5".into(),
            access_token: Secret::new("TOKEN123"),
            xuid: Some("2535".into()),
        }
    }

    fn opts() -> LaunchOptions {
        LaunchOptions {
            game_dir: PathBuf::from("G"),
            java: PathBuf::from("java"),
            java_major: Some(21),
            memory_min_mb: 512,
            memory_max_mb: 4096,
            resolution: None,
            fullscreen: false,
            jvm_args: vec![],
            launcher_name: "rustic-launcher".into(),
            launcher_version: "0.2.7".into(),
            quick_play_server: None,
        }
    }

    fn installed(v: serde_json::Value) -> Installed {
        let version: VersionJson = serde_json::from_value(v).unwrap();
        Installed {
            asset_index: version.asset_index.as_ref().map(|a| a.id.clone()),
            logging_config: version
                .logging
                .as_ref()
                .and_then(|l| l.client.as_ref())
                .map(|c| Path::new("assets/log_configs").join(&c.file.id)),
            version,
            libraries: LibraryPlan {
                classpath: vec![PathBuf::from("libraries/a/a.jar")],
                natives: vec![],
                downloads: vec![],
            },
            client_jar: Some(PathBuf::from("versions/9.2/9.2.jar")),
            client_source: None,
        }
    }

    /// The shape of a recent version file (written for this test).
    fn modern() -> serde_json::Value {
        json!({
            "id": "9.2", "type": "release", "mainClass": "net.minecraft.client.main.Main",
            "assetIndex": {"id": "30", "url": "https://piston-meta.mojang.com/30.json"},
            "logging": {"client": {"argument": "-Dlog4j.configurationFile=${path}", "type": "log4j2-xml",
                "file": {"id": "client-1.21.2.xml", "sha1": "s", "url": "https://piston-data.mojang.com/x.xml"}}},
            "arguments": {
                "default-user-jvm": [
                    {"value": ["-Xms2G", "-Xmx4G", "-XX:+UseStringDeduplication"]},
                    {"rules": [{"action": "allow", "os": {"name": "windows", "versionRange": {"min": "10.0.17134"}}}], "value": ["-XX:+UseZGC"]},
                    {"rules": [{"action": "allow", "os": {"name": "windows", "versionRange": {"max": "10.0.17134"}}}], "value": ["-XX:+UseG1GC"]}
                ],
                "game": ["--username", "${auth_player_name}", "--version", "${version_name}", "--gameDir", "${game_directory}",
                    "--assetsDir", "${assets_root}", "--assetIndex", "${assets_index_name}", "--uuid", "${auth_uuid}",
                    "--accessToken", "${auth_access_token}", "--xuid", "${auth_xuid}", "--versionType", "${version_type}",
                    {"rules": [{"action": "allow", "features": {"is_demo_user": true}}], "value": "--demo"},
                    {"rules": [{"action": "allow", "features": {"has_custom_resolution": true}}], "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"]},
                    {"rules": [{"action": "allow", "features": {"is_quick_play_multiplayer": true}}], "value": ["--quickPlayMultiplayer", "${quickPlayMultiplayer}"]}],
                "jvm": [{"rules": [{"action": "allow", "os": {"name": "osx"}}], "value": ["-XstartOnFirstThread"]},
                    "-Djava.library.path=${natives_directory}/java", "-Dminecraft.launcher.brand=${launcher_name}", "-cp", "${classpath}"]
            }
        })
    }

    fn after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    }

    #[test]
    fn modern_arguments() {
        let root = Path::new("R");
        let mut o = opts();
        o.resolution = Some((1280, 720));
        o.quick_play_server = Some("play.example.org".into());
        let cmd = build(
            &installed(modern()),
            root,
            Path::new("N"),
            &auth(),
            &o,
            &win(),
        )
        .unwrap();
        let a = &cmd.args;
        assert_eq!(a[0], "-Xms512M");
        assert_eq!(a[1], "-Xmx4096M");
        // The version's defaults, without its memory and with this system's collector.
        assert!(a.contains(&"-XX:+UseStringDeduplication".into()));
        assert!(a.contains(&"-XX:+UseZGC".into()));
        assert!(!a.contains(&"-XX:+UseG1GC".into()));
        assert!(!a.iter().any(|x| x == "-Xmx4G" || x == "-Xms2G"));
        assert!(!a.contains(&"-XstartOnFirstThread".into()));
        assert_eq!(
            a.iter()
                .find(|x| x.starts_with("-Djava.library.path"))
                .unwrap(),
            &format!(
                "-Djava.library.path={}",
                Path::new("N").display().to_string() + "/java"
            )
        );
        let cp = after(a, "-cp").unwrap();
        assert_eq!(
            cp,
            format!(
                "{};{}",
                root.join("libraries/a/a.jar").display(),
                root.join("versions/9.2/9.2.jar").display()
            )
        );
        assert!(a.iter().any(
            |x| x.starts_with("-Dlog4j.configurationFile=") && x.ends_with("client-1.21.2.xml")
        ));
        let main = a
            .iter()
            .position(|x| x == "net.minecraft.client.main.Main")
            .unwrap();
        assert!(a.iter().position(|x| x == "-cp").unwrap() < main);
        assert_eq!(after(a, "--username"), Some("Alex"));
        assert_eq!(after(a, "--accessToken"), Some("TOKEN123"));
        assert_eq!(after(a, "--xuid"), Some("2535"));
        assert_eq!(after(a, "--assetIndex"), Some("30"));
        assert_eq!(after(a, "--versionType"), Some("release"));
        assert_eq!(after(a, "--width"), Some("1280"));
        assert_eq!(after(a, "--quickPlayMultiplayer"), Some("play.example.org"));
        assert!(!a.contains(&"--demo".into()));
        assert!(!a.iter().any(|x| x.contains("${")));
        // The token never shows in what is logged.
        assert!(!cmd.display().contains("TOKEN123"));
        assert!(cmd.display().contains("********"));
        assert!(cmd.use_argfile);
    }

    #[test]
    fn custom_jvm_arguments_replace_the_defaults() {
        let mut o = opts();
        o.jvm_args = vec!["-XX:+UseG1GC".into()];
        let cmd = build(
            &installed(modern()),
            Path::new("R"),
            Path::new("N"),
            &auth(),
            &o,
            &win(),
        )
        .unwrap();
        assert!(cmd.args.contains(&"-XX:+UseG1GC".into()));
        assert!(!cmd.args.contains(&"-XX:+UseZGC".into()));
    }

    #[test]
    fn legacy_arguments() {
        // The shape of a pre-1.13 version file (written for this test).
        let v = json!({
            "id": "1.8.9", "type": "release", "mainClass": "net.minecraft.client.main.Main",
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type}",
            "assetIndex": {"id": "1.8", "url": "https://piston-meta.mojang.com/1.8.json"}
        });
        let mut o = opts();
        o.java_major = Some(8);
        o.resolution = Some((854, 480));
        let cmd = build(
            &installed(v),
            Path::new("R"),
            Path::new("N"),
            &auth(),
            &o,
            &win(),
        )
        .unwrap();
        let a = &cmd.args;
        assert!(a.iter().any(|x| x.starts_with("-XX:HeapDumpPath=")));
        assert_eq!(
            a.iter()
                .find(|x| x.starts_with("-Djava.library.path"))
                .unwrap(),
            &format!("-Djava.library.path={}", Path::new("N").display())
        );
        assert_eq!(after(a, "--userProperties"), Some("{}"));
        assert_eq!(after(a, "--userType"), Some("msa"));
        assert_eq!(after(a, "--width"), Some("854"));
        assert!(!cmd.use_argfile);
    }

    #[test]
    fn very_old_session_argument() {
        let v = json!({
            "id": "b1.7.3", "type": "old_beta", "mainClass": "net.minecraft.launchwrapper.Launch",
            "minecraftArguments": "${auth_player_name} ${auth_session} --gameDir ${game_directory} --assetsDir ${game_assets}",
            "assetIndex": {"id": "pre-1.6", "url": "https://launchermeta.mojang.com/pre-1.6.json"}
        });
        let cmd = build(
            &installed(v),
            Path::new("R"),
            Path::new("N"),
            &auth(),
            &opts(),
            &win(),
        )
        .unwrap();
        let main = cmd
            .args
            .iter()
            .position(|a| a == "net.minecraft.launchwrapper.Launch")
            .unwrap();
        assert_eq!(cmd.args[main + 1], "Alex");
        assert_eq!(
            cmd.args[main + 2],
            "token:TOKEN123:069a79f444e94726a5befca90e38aaf5"
        );
        // No index on disk here: `${game_assets}` falls back to the assets folder.
        assert_eq!(
            after(&cmd.args, "--assetsDir").map(PathBuf::from),
            Some(Path::new("R").join("assets"))
        );
        assert!(!cmd.display().contains("TOKEN123"));
    }

    #[test]
    fn substitution_and_argfile() {
        let mut vars: Vars = HashMap::new();
        vars.insert("a", "1".into());
        assert_eq!(substitute("x${a}y${b}z${", &vars), "x1y${b}z${");
        let cmd = LaunchCommand {
            java: "java".into(),
            args: vec!["-cp".into(), "C:\\a b\\c.jar".into(), "say \"hi\"".into()],
            cwd: ".".into(),
            secrets: vec![],
            use_argfile: true,
        };
        assert_eq!(
            cmd.argfile_text(),
            "\"-cp\"\n\"C:\\\\a b\\\\c.jar\"\n\"say \\\"hi\\\"\"\n"
        );
    }

    #[test]
    fn prepare_unpacks_natives_and_checks_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let jar = root.join("libraries/n/n.jar");
        std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
        {
            let f = std::fs::File::create(&jar).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("lwjgl.dll", o).unwrap();
            std::io::Write::write_all(&mut z, b"dll").unwrap();
            z.start_file("META-INF/MANIFEST.MF", o).unwrap();
            z.start_file("../evil.dll", o).unwrap();
            z.finish().unwrap();
        }
        let mut inst = installed(json!({"id": "x", "mainClass": "M"}));
        inst.libraries = LibraryPlan {
            classpath: vec![PathBuf::from("libraries/n/n.jar")],
            natives: vec![NativeJar {
                path: PathBuf::from("libraries/n/n.jar"),
                exclude: vec!["META-INF/".into()],
            }],
            downloads: vec![],
        };
        inst.client_jar = None;
        let natives = root.join("natives/p");
        prepare(&inst, root, &natives, &root.join("game")).unwrap();
        assert_eq!(std::fs::read(natives.join("lwjgl.dll")).unwrap(), b"dll");
        assert!(!natives.join("META-INF").exists());
        assert!(!root.join("natives/evil.dll").exists());
        inst.client_jar = Some(PathBuf::from("versions/x/x.jar"));
        assert!(matches!(
            prepare(&inst, root, &natives, &root.join("game")),
            Err(JavaError::MissingFile(_))
        ));
    }
}
