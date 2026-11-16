//! Bedrock Edition on Windows: detected as its Microsoft Store package
//! (`Microsoft.MinecraftUWP_8wekyb3d8bbwe`), started through the shell with its app id, or the
//! Store's page opened when it is missing. Other systems have no Bedrock Edition for PC.

use crate::{JavaError, Result};
use serde::Serialize;

/// The package family of Bedrock Edition for Windows.
pub const PACKAGE_FAMILY: &str = "Microsoft.MinecraftUWP_8wekyb3d8bbwe";
/// The Microsoft Store product of Minecraft for Windows.
pub const STORE_PRODUCT_ID: &str = "9NBLGGH2JHXJ";
/// The Store app's page link (opens the Microsoft Store app).
pub const STORE_URL: &str = "ms-windows-store://pdp/?ProductId=9NBLGGH2JHXJ";
/// The same page on the web (when the Store app can't open).
pub const STORE_WEB_URL: &str = "https://apps.microsoft.com/detail/9NBLGGH2JHXJ";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BedrockStatus {
    /// Windows: Bedrock Edition exists for this system.
    pub supported: bool,
    pub installed: bool,
    pub version: Option<String>,
    /// The package's application id (`App`), used to start it.
    #[serde(skip)]
    pub app_id: Option<String>,
}

/// Reads `Name|Version|AppId` printed by the detection script.
pub fn parse_detection(output: &str) -> BedrockStatus {
    let line = output
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("Microsoft.MinecraftUWP|"));
    let Some(line) = line else {
        return BedrockStatus {
            supported: true,
            installed: false,
            version: None,
            app_id: None,
        };
    };
    let mut parts = line.split('|');
    let _name = parts.next();
    let version = parts.next().map(str::to_owned).filter(|v| !v.is_empty());
    let app_id = parts
        .next()
        .map(str::trim)
        .filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'))
        .map(str::to_owned);
    BedrockStatus {
        supported: true,
        installed: true,
        version,
        app_id,
    }
}

/// Whether Bedrock Edition is installed (Windows; elsewhere `supported: false`).
pub async fn detect() -> BedrockStatus {
    if !cfg!(windows) {
        return BedrockStatus {
            supported: false,
            installed: false,
            version: None,
            app_id: None,
        };
    }
    let script = "$p = Get-AppxPackage -Name 'Microsoft.MinecraftUWP' | Select-Object -First 1; \
        if ($p) { $id = ''; try { $id = (Get-AppxPackageManifest $p).Package.Applications.Application.Id | Select-Object -First 1 } catch {}; \
        Write-Output ('Microsoft.MinecraftUWP|' + $p.Version + '|' + $id) }";
    let mut cmd = tokio::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    crate::process::no_window(&mut cmd);
    match tokio::time::timeout(std::time::Duration::from_secs(15), cmd.output()).await {
        Ok(Ok(out)) => parse_detection(&String::from_utf8_lossy(&out.stdout)),
        _ => {
            // Fallback: the package's data folder.
            let installed = std::env::var_os("LOCALAPPDATA").is_some_and(|d| {
                std::path::Path::new(&d)
                    .join("Packages")
                    .join(PACKAGE_FAMILY)
                    .is_dir()
            });
            BedrockStatus {
                supported: true,
                installed,
                version: None,
                app_id: None,
            }
        }
    }
}

/// Starts Bedrock Edition (`shell:AppsFolder\<family>!<app>`).
pub async fn launch(status: &BedrockStatus) -> Result<()> {
    if !status.supported {
        return Err(JavaError::Bedrock("Windows only".into()));
    }
    if !status.installed {
        return Err(JavaError::Bedrock("not installed".into()));
    }
    let app = status.app_id.as_deref().unwrap_or("App");
    let target = format!("shell:AppsFolder\\{PACKAGE_FAMILY}!{app}");
    let mut cmd = tokio::process::Command::new("explorer.exe");
    cmd.arg(target).stdin(std::process::Stdio::null());
    cmd.spawn()
        .map_err(|e| JavaError::Bedrock(format!("could not start it: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_detection_output() {
        let s = parse_detection("\r\nMicrosoft.MinecraftUWP|1.26.6301.0|App\r\n");
        assert!(s.installed && s.supported);
        assert_eq!(s.version.as_deref(), Some("1.26.6301.0"));
        assert_eq!(s.app_id.as_deref(), Some("App"));
        assert!(!parse_detection("").installed);
        assert!(
            parse_detection("Microsoft.MinecraftUWP|1.0|bad id;x")
                .app_id
                .is_none()
        );
    }
}
