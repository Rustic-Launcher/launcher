//! A version file (`versions/<id>/<id>.json`): Mojang's, or a loader's profile that inherits from
//! one. Only the fields the launcher uses are typed; the format is Mojang's public one.

use crate::rules::Rule;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits_from: Option<String>,
    /// The version whose client jar this one runs (old Forge profiles name it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    /// Old versions (before 1.13): the game arguments as one string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_arguments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<Downloads>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_version: Option<JavaVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
    /// The memory and garbage collector defaults the official launcher offers (recent files).
    #[serde(
        default,
        rename = "default-user-jvm",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub default_user_jvm: Vec<Argument>,
}

/// One argument, or several under rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Ruled {
        #[serde(default)]
        rules: Vec<Rule>,
        value: OneOrMany,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    pub fn values(&self) -> Vec<String> {
        match self {
            Self::One(s) => vec![s.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    /// `group:artifact:version[:classifier][@extension]`.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<LibraryDownloads>,
    /// A Maven repository base (loader profiles: Fabric, Quilt, old Forge).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Old versions: OS name → natives classifier (`natives-windows`, `natives-windows-${arch}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natives: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<Extract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifiers: Option<BTreeMap<String, Artifact>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Empty for files a loader's installer generates (nothing to download).
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Downloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<Artifact>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub major_version: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Logging {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<LoggingClient>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LoggingClient {
    /// `-Dlog4j.configurationFile=${path}`.
    pub argument: String,
    pub file: LoggingFile,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub sha1: String,
    #[serde(default)]
    pub size: Option<u64>,
    pub url: String,
}

impl VersionJson {
    pub fn parse(text: &str) -> crate::Result<Self> {
        serde_json::from_str(text).map_err(|e| crate::JavaError::bad(e.to_string()))
    }

    /// The Java runtime component the version asks for (`java-runtime-delta`), else the one of its
    /// major (files without `javaVersion` are old: Java 8).
    pub fn runtime_component(&self) -> (String, u32) {
        let jv = self.java_version.clone().unwrap_or_default();
        let major = jv.major_version.unwrap_or(8);
        let component = jv
            .component
            .unwrap_or_else(|| component_for_major(major).to_owned());
        (component, major)
    }
}

/// Mojang's runtime component for a Java major, for files that name only the major.
pub fn component_for_major(major: u32) -> &'static str {
    match major {
        0..=8 => "jre-legacy",
        9..=16 => "java-runtime-alpha",
        17 => "java-runtime-gamma",
        18..=21 => "java-runtime-delta",
        _ => "java-runtime-epsilon",
    }
}

/// Whether `id` is safe as a folder and file name (`versions/<id>/<id>.json`).
pub fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_argument_shapes() {
        let v = VersionJson::parse(
            r#"{"id":"x","arguments":{"game":["--a",{"rules":[],"value":"b"},{"rules":[],"value":["c","d"]}],"jvm":[]},"libraries":[]}"#,
        )
        .unwrap();
        let game = &v.arguments.unwrap().game;
        assert_eq!(game[0], Argument::Plain("--a".into()));
        assert!(
            matches!(&game[2], Argument::Ruled { value: OneOrMany::Many(v), .. } if v.len() == 2)
        );
    }

    #[test]
    fn runtime_components() {
        let mut v = VersionJson::default();
        assert_eq!(v.runtime_component(), ("jre-legacy".into(), 8));
        v.java_version = Some(JavaVersion {
            component: None,
            major_version: Some(21),
        });
        assert_eq!(v.runtime_component().0, "java-runtime-delta");
        v.java_version = Some(JavaVersion {
            component: Some("java-runtime-epsilon".into()),
            major_version: Some(25),
        });
        assert_eq!(v.runtime_component(), ("java-runtime-epsilon".into(), 25));
    }

    #[test]
    fn safe_ids() {
        assert!(safe_id("1.21.1"));
        assert!(safe_id("fabric-loader-0.17.2-1.21.1"));
        assert!(safe_id("26.4-snapshot-8"));
        assert!(!safe_id("../x"));
        assert!(!safe_id("a/b"));
        assert!(!safe_id(".."));
        assert!(!safe_id(""));
    }
}
