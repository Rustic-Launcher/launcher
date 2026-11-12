//! Signed checksums, as Horizon's releases carry them (horizon/docs/release-flow.md, "Signed
//! checksums"; crates/horizon-release-sign): `SHA256SUMS` in `sha256sum`'s format, and a
//! signature file with one base64 Ed25519 signature per line over `<namespace> <version>\n`
//! followed by SHA256SUMS's bytes. Horizon's namespace is `horizon release`; a Nightly is
//! signed for its own version `nightly-<date>-<commit>` in `SHA256SUMS.v2.sig` (what current
//! Horizon launchers require), a numbered release for its tag in `SHA256SUMS.sig`.
//! Rustic's own builds use the same layout under the `rustic release` namespace (ADR 0045).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ring::signature::{ED25519, UnparsedPublicKey};

/// An Ed25519 public key.
pub type PublicKey = [u8; 32];

/// Horizon's release keys, built in.
pub const HORIZON_KEYS: &str = include_str!("horizon_release_keys.txt");

/// The keys of a keys file: one base64 key per line, `#` comments and blank lines skipped.
pub fn parse_keys(text: &str) -> Result<Vec<PublicKey>, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(parse_key)
        .collect()
}

/// One base64 key.
pub fn parse_key(line: &str) -> Result<PublicKey, String> {
    let bytes = STANDARD
        .decode(line.trim())
        .map_err(|e| format!("{line}: not base64 ({e})"))?;
    PublicKey::try_from(bytes.as_slice()).map_err(|_| format!("{line}: not an Ed25519 key"))
}

/// What a signature covers.
pub fn message(namespace: &str, signed_for: &str, sums: &[u8]) -> Vec<u8> {
    let mut m = format!("{namespace} {signed_for}\n").into_bytes();
    m.extend_from_slice(sums);
    m
}

/// `(file name, sha256 hex)` pairs of a SHA256SUMS file.
pub fn parse_sums(text: &str) -> Result<Vec<(String, String)>, String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let (hex, name) = line
                .split_once(' ')
                .ok_or_else(|| format!("SHA256SUMS: {line:?} is not `<sha256>  <file>`"))?;
            let name = name.strip_prefix([' ', '*']).unwrap_or(name);
            if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) || name.is_empty() {
                return Err(format!("SHA256SUMS: {line:?} is not `<sha256>  <file>`"));
            }
            Ok((name.to_owned(), hex.to_ascii_lowercase()))
        })
        .collect()
}

/// The signed SHA-256 of `asset`, once one of `keys` is found to have signed `sums` for
/// `signed_for`; why the download must be refused otherwise.
pub fn signed_digest(
    namespace: &str,
    signed_for: &str,
    asset: &str,
    sums: &[u8],
    signature: &str,
    keys: &[PublicKey],
) -> Result<String, String> {
    if keys.is_empty() {
        return Err(
            "No release key is configured for this build source, so its downloads can't be checked: nothing was installed."
                .into(),
        );
    }
    let message = message(namespace, signed_for, sums);
    let signed = signature
        .lines()
        .filter_map(|l| STANDARD.decode(l.trim()).ok())
        .any(|sig| {
            keys.iter().any(|k| {
                UnparsedPublicKey::new(&ED25519, k)
                    .verify(&message, &sig)
                    .is_ok()
            })
        });
    if !signed {
        return Err(format!(
            "{signed_for}'s signature doesn't match the release key, so it was not installed. The download may have been tampered with."
        ));
    }
    let text = std::str::from_utf8(sums).map_err(|_| "SHA256SUMS is not text".to_owned())?;
    parse_sums(text)?
        .into_iter()
        .find(|(n, _)| n == asset)
        .map(|(_, hex)| hex)
        .ok_or_else(|| format!("{asset} is not in {signed_for}'s signed checksums: not installed."))
}

/// The version a Nightly installs as, from its GitHub title (`Nightly 2026-10-28 · 0c02067` →
/// `nightly-2026-10-28-0c02067`), strictly (horizon-release-sign `nightly_title_version`): a
/// title without its date and commit gives `None`, and such a Nightly can't be verified.
pub fn nightly_title_version(title: &str) -> Option<String> {
    let (date, commit) = title.strip_prefix("Nightly ")?.split_once(" · ")?;
    let commit_ok = (7..=40).contains(&commit.len())
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    (is_date(date) && commit_ok).then(|| format!("nightly-{date}-{commit}"))
}

fn is_date(text: &str) -> bool {
    text.len() == 10
        && text.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

#[cfg(test)]
pub(crate) mod test_keys {
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    /// A throwaway key pair: (signer, base64 public key).
    pub fn generate() -> (Ed25519KeyPair, String) {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            pair.public_key().as_ref(),
        );
        (pair, public)
    }
}

/// Signs `sums` (tests and the release tooling's docs): one base64 signature line.
pub fn sign_with(
    pair: &ring::signature::Ed25519KeyPair,
    namespace: &str,
    signed_for: &str,
    sums: &[u8],
) -> String {
    STANDARD.encode(pair.sign(&message(namespace, signed_for, sums)).as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizon_keys_parse() {
        assert_eq!(parse_keys(HORIZON_KEYS).unwrap().len(), 1);
    }

    #[test]
    fn signatures_are_bound_to_the_version_and_namespace() {
        let (pair, public) = test_keys::generate();
        let keys = vec![parse_key(&public).unwrap()];
        let sums = format!("{}  horizon-windows-x64.zip\n", "a".repeat(64));
        let sig = sign_with(
            &pair,
            "horizon release",
            "nightly-2026-10-28-0c02067",
            sums.as_bytes(),
        );
        assert_eq!(
            signed_digest(
                "horizon release",
                "nightly-2026-10-28-0c02067",
                "horizon-windows-x64.zip",
                sums.as_bytes(),
                &sig,
                &keys
            )
            .unwrap(),
            "a".repeat(64)
        );
        // Another version, another namespace, another key, a changed list: refused.
        assert!(
            signed_digest(
                "horizon release",
                "nightly-2026-10-27-0c02067",
                "horizon-windows-x64.zip",
                sums.as_bytes(),
                &sig,
                &keys
            )
            .is_err()
        );
        assert!(
            signed_digest(
                "rustic release",
                "nightly-2026-10-28-0c02067",
                "horizon-windows-x64.zip",
                sums.as_bytes(),
                &sig,
                &keys
            )
            .is_err()
        );
        let (_, other) = test_keys::generate();
        assert!(
            signed_digest(
                "horizon release",
                "nightly-2026-10-28-0c02067",
                "horizon-windows-x64.zip",
                sums.as_bytes(),
                &sig,
                &[parse_key(&other).unwrap()]
            )
            .is_err()
        );
        let tampered = sums.replace('a', "b");
        assert!(
            signed_digest(
                "horizon release",
                "nightly-2026-10-28-0c02067",
                "horizon-windows-x64.zip",
                tampered.as_bytes(),
                &sig,
                &keys
            )
            .is_err()
        );
        assert!(signed_digest("horizon release", "x", "a", b"", "", &[]).is_err());
    }

    #[test]
    fn nightly_titles() {
        assert_eq!(
            nightly_title_version("Nightly 2026-10-28 · 0c02067").as_deref(),
            Some("nightly-2026-10-28-0c02067")
        );
        assert_eq!(nightly_title_version("Nightly 2026-10-28"), None);
        assert_eq!(nightly_title_version("Nightly 2026-10-28 · XYZ1234"), None);
    }
}
