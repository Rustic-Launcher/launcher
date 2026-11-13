//! A real edition source archive (`rusticmc-native pack-source`) unpacks into a tree the build
//! accepts. Run with the archive's path:
//! `RUSTIC_NATIVE_TEST_ARCHIVE=src.tar.gz cargo test -p rusticmc-native --test real_source -- --ignored`
use rusticmc_native::source;

#[test]
#[ignore = "needs a packed source archive (RUSTIC_NATIVE_TEST_ARCHIVE)"]
fn a_packed_edition_source_unpacks_into_a_buildable_tree() {
    let archive =
        std::env::var_os("RUSTIC_NATIVE_TEST_ARCHIVE").expect("RUSTIC_NATIVE_TEST_ARCHIVE");
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("src");
    source::unpack(std::path::Path::new(&archive), &dir, source::MAX_UNPACKED).unwrap();
    source::check_layout(&dir).unwrap();
    let m: source::SourceManifest =
        serde_json::from_slice(&std::fs::read(dir.join(source::MANIFEST)).unwrap()).unwrap();
    assert_eq!(m.format, 1);
    assert!(
        dir.join(source::HORIZON)
            .join("crates/horizon-client/Cargo.toml")
            .is_file()
    );
    assert!(!dir.join(source::HORIZON).join("target").exists());
}
