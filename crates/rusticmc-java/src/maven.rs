//! Maven coordinates (`group:artifact:version[:classifier][@extension]`) as library paths.

use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coordinate {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl Coordinate {
    pub fn parse(name: &str) -> Option<Self> {
        let (main, extension) = match name.split_once('@') {
            Some((m, e)) => (m, e.to_owned()),
            None => (name, "jar".to_owned()),
        };
        let mut parts = main.split(':');
        let group = parts.next()?.to_owned();
        let artifact = parts.next()?.to_owned();
        let version = parts.next()?.to_owned();
        let classifier = parts.next().map(str::to_owned);
        if parts.next().is_some() {
            return None;
        }
        let ok = |s: &str| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        };
        let all = [&group, &artifact, &version, &extension];
        if !all.iter().all(|s| ok(s)) || classifier.as_deref().is_some_and(|c| !ok(c)) {
            return None;
        }
        Some(Self {
            group,
            artifact,
            version,
            classifier,
            extension,
        })
    }

    /// `group/as/path/artifact/version/artifact-version[-classifier].ext`, with `/`.
    pub fn path(&self) -> String {
        let file = match &self.classifier {
            Some(c) => format!("{}-{}-{c}.{}", self.artifact, self.version, self.extension),
            None => format!("{}-{}.{}", self.artifact, self.version, self.extension),
        };
        format!(
            "{}/{}/{}/{file}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        )
    }

    /// What two libraries must share to be the same one (`group:artifact[:classifier]`), so a
    /// loader's newer copy replaces the game's.
    pub fn identity(&self) -> String {
        match &self.classifier {
            Some(c) => format!("{}:{}:{c}", self.group, self.artifact),
            None => format!("{}:{}", self.group, self.artifact),
        }
    }
}

/// A relative path from a file, refused when it could leave its folder.
pub fn safe_relative(path: &str) -> Option<PathBuf> {
    let ok = !path.is_empty()
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
    ok.then(|| PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates() {
        let c = Coordinate::parse("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap();
        assert_eq!(
            c.path(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        );
        assert_eq!(c.identity(), "org.lwjgl:lwjgl:natives-windows");
        let z = Coordinate::parse("de.oceanlabs.mcp:mcp_config:1.20.1@zip").unwrap();
        assert_eq!(
            z.path(),
            "de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1.zip"
        );
        assert_eq!(
            Coordinate::parse("net.fabricmc:fabric-loader:0.17.2")
                .unwrap()
                .identity(),
            "net.fabricmc:fabric-loader"
        );
        assert!(Coordinate::parse("a:b").is_none());
        assert!(Coordinate::parse("a:..:1").is_none());
        assert!(Coordinate::parse("a:b:1:c:d").is_none());
        assert!(Coordinate::parse("a/../b:c:1").is_none());
    }

    #[test]
    fn relative_paths() {
        assert!(safe_relative("a/b.jar").is_some());
        assert!(safe_relative("../b.jar").is_none());
        assert!(safe_relative("/b.jar").is_none());
        assert!(safe_relative("a\\b.jar").is_none());
    }
}
