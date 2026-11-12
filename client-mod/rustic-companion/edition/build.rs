//! The game's main thread needs 8 MB of stack on Windows as on macOS and Linux (Windows gives 1
//! MB), as Horizon's executables set it (horizon/packaging/windows/exe.rs). On Windows the
//! executable also carries Rustic's icon (the launcher's, `apps/launcher/src-tauri/icons/icon.ico`)
//! as icon resource 1, for Explorer and shortcuts; the window takes Rustic's at run time
//! (`main`, ADR 0148).
fn main() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os == "windows" {
        let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let arg = if env == "msvc" {
            "/STACK:8388608"
        } else {
            "-Wl,--stack,8388608"
        };
        println!("cargo:rustc-link-arg-bin=rustic-client={arg}");
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let ico = manifest.join("../../../apps/launcher/src-tauri/icons/icon.ico");
        println!("cargo:rerun-if-changed={}", ico.display());
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(&ico.to_string_lossy());
        // A machine without a resource compiler still builds, without the file's icon (the
        // window keeps Rustic's); CI always has one.
        if let Err(e) = resource.compile() {
            if std::env::var_os("CI").is_some() {
                panic!("embedding Rustic's icon: {e}");
            }
            println!("cargo:warning=building without Rustic's file icon: {e}");
        }
    }
}
