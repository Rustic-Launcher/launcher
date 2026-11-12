//! POSIX shell quoting. SSH's `exec` sends one string that the remote login shell parses, so every
//! value that comes from the user or from the network (server names, player names, console lines,
//! paths, URLs) goes through [`quote`] before it is put in a command. Nothing else builds commands.
//!
//! Single quotes are the only POSIX quoting with no special characters inside: `'...'` keeps every
//! byte literally, and a `'` is written as `'\''` (close, escaped quote, reopen). NUL bytes can't be
//! passed to a program at all, so they are refused instead of silently cut.

use crate::error::{HostingError, Result};

/// Characters that never need quoting in any POSIX shell (kept to a conservative set, so the
/// output stays readable in logs: `/srv/rustic/servers/survival`, `--port`, `25565`).
fn is_plain(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':' | '=' | '@' | ',' | '+')
}

/// Quote one word for a POSIX shell. Refuses NUL (a C string ends there; a quoted NUL would be
/// truncated by the remote `execve`, changing the meaning silently).
pub fn quote(word: &str) -> Result<String> {
    if word.contains('\0') {
        return Err(HostingError::Invalid(
            "a value sent to the server contains a NUL byte".into(),
        ));
    }
    // Quoting never stops a program from reading `-x` as an option (it sees the same bytes), so a
    // leading dash stays plain; callers put a path before user values where it matters.
    if !word.is_empty() && word.chars().all(is_plain) {
        return Ok(word.to_owned());
    }
    let mut out = String::with_capacity(word.len() + 2);
    out.push('\'');
    for c in word.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    Ok(out)
}

/// Quote every word and join them with spaces: a full command line.
pub fn join<I, S>(words: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut parts = Vec::new();
    for w in words {
        parts.push(quote(w.as_ref())?);
    }
    Ok(parts.join(" "))
}

/// A command line built word by word. Literal shell syntax (`&&`, `|`, `2>&1`) is only added with
/// [`Cmd::raw`], from constants in this crate, never from input.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cmd {
    text: String,
}

impl Cmd {
    pub fn new(program: &str) -> Result<Self> {
        let mut c = Self::default();
        c.push_arg(program)?;
        Ok(c)
    }

    /// Append one argument, quoted.
    pub fn arg(mut self, word: impl AsRef<str>) -> Result<Self> {
        self.push_arg(word.as_ref())?;
        Ok(self)
    }

    /// Append several arguments, each quoted.
    pub fn args<I, S>(mut self, words: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for w in words {
            self.push_arg(w.as_ref())?;
        }
        Ok(self)
    }

    /// Append literal shell text (operators, redirections). Only ever call it with constants.
    pub fn raw(mut self, text: &'static str) -> Self {
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(text);
        self
    }

    /// `self && other`.
    pub fn and(self, other: Cmd) -> Self {
        let mut s = self.raw("&&");
        s.text.push(' ');
        s.text.push_str(&other.text);
        s
    }

    fn push_arg(&mut self, word: &str) -> Result<()> {
        let q = quote(word)?;
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(&q);
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn into_string(self) -> String {
        self.text
    }

    /// Wrap a script so it runs under `sh -c` (for `sudo`/`su`, which take one command).
    pub fn script(script: &str) -> Result<Self> {
        Self::new("sh")?.arg("-c")?.arg(script)
    }
}

impl std::fmt::Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// A name usable as a server folder, a unit name and a user name: lowercase letters, digits and
/// dashes, 1 to 32 characters, starting with a letter. Checked before anything reaches a shell.
pub fn validate_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 32
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(HostingError::Invalid(format!(
            "`{name}` is not a valid name: use 1 to 32 lowercase letters, digits and dashes, starting with a letter"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn plain_words_stay_readable() {
        assert_eq!(
            quote("/srv/rustic/servers/a-1").unwrap(),
            "/srv/rustic/servers/a-1"
        );
        assert_eq!(quote("engine.path=/x/y").unwrap(), "engine.path=/x/y");
        assert_eq!(quote("25565").unwrap(), "25565");
    }

    #[test]
    fn specials_are_single_quoted() {
        assert_eq!(quote("").unwrap(), "''");
        assert_eq!(quote("a b").unwrap(), "'a b'");
        assert_eq!(quote("$(reboot)").unwrap(), "'$(reboot)'");
        assert_eq!(quote("`id`").unwrap(), "'`id`'");
        assert_eq!(quote("a;b&&c|d").unwrap(), "'a;b&&c|d'");
        assert_eq!(quote("it's").unwrap(), "'it'\\''s'");
        assert_eq!(quote("'").unwrap(), "''\\'''");
        assert_eq!(quote("-rf").unwrap(), "-rf");
        assert_eq!(quote("~root").unwrap(), "'~root'");
        assert_eq!(quote("*").unwrap(), "'*'");
        assert_eq!(quote("a\nb").unwrap(), "'a\nb'");
        assert_eq!(quote("héllo").unwrap(), "'héllo'");
    }

    #[test]
    fn nul_is_refused() {
        assert!(quote("a\0b").is_err());
        assert!(Cmd::new("echo").unwrap().arg("x\0").is_err());
    }

    #[test]
    fn cmd_builder_quotes_args_not_operators() {
        let c = Cmd::new("horizon-server")
            .unwrap()
            .args(["--dir", "/srv/my server", "cmd", "say", "hi; rm -rf /"])
            .unwrap()
            .raw("2>&1");
        assert_eq!(
            c.as_str(),
            "horizon-server --dir '/srv/my server' cmd say 'hi; rm -rf /' 2>&1"
        );
        let both = Cmd::new("true").unwrap().and(Cmd::new("false").unwrap());
        assert_eq!(both.as_str(), "true && false");
        let s = Cmd::script("echo 'x'").unwrap();
        assert_eq!(s.as_str(), "sh -c 'echo '\\''x'\\'''");
    }

    #[test]
    fn names() {
        assert!(validate_name("survival-2").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("2fast").is_err());
        assert!(validate_name("Upper").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("a b").is_err());
        assert!(validate_name(&"a".repeat(33)).is_err());
    }

    /// The real proof: a POSIX shell gives back exactly the bytes we quoted. Runs where `sh` exists
    /// (Linux CI, Git Bash on Windows); skipped silently elsewhere.
    #[test]
    fn round_trips_through_a_real_shell() {
        let Ok(probe) = Command::new("sh").arg("-c").arg("printf ok").output() else {
            return;
        };
        if probe.stdout != b"ok" {
            return;
        }
        let nasty = [
            "plain",
            "with space",
            "it's",
            "''",
            "\"double\"",
            "$HOME",
            "$(id)",
            "`id`",
            "a\\b",
            "semi;colon",
            "pipe|&amp",
            "new\nline",
            "tab\there",
            "*?[a]",
            "-n",
            "~",
            "!bang",
            "ünïcode ✓",
            "",
        ];
        for word in nasty {
            let script = format!("printf %s {}", quote(word).unwrap());
            let out = Command::new("sh").arg("-c").arg(&script).output().unwrap();
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                word,
                "round trip of {word:?} through `{script}`"
            );
        }
    }
}
