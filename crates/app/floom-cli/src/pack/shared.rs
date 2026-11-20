// SPDX-License-Identifier: MIT
//! `cargo metadata` inspection, shared by every platform packer.
//!
//! The CLI is std-only, so instead of `serde_json` this holds a small recursive
//! JSON reader that pulls out just the four fields packaging needs. Package
//! resolution runs `cargo metadata --no-deps` and matches the package whose
//! directory is `dir`, so `floom package <DIR>` works from inside a workspace
//! too.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{PackResult, Tier};
use crate::toolchain::SdkPrecondition;

/// A resolved Cargo project, ready to package.
pub struct Project {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub package_name: String,
    /// Feature names this package declares. `default_features` derives the
    /// build's features from these; the platform is only known to the packer.
    pub features: Vec<String>,
}

/// Inspect the Cargo project rooted at `dir`.
pub fn inspect(dir: &Path) -> Result<Project, String> {
    let manifest = dir.join("Cargo.toml");
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .map_err(|e| format!("cannot run `cargo metadata`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed ({}) for {}",
            output.status,
            manifest.display()
        ));
    }
    let json = String::from_utf8(output.stdout)
        .map_err(|e| format!("cargo metadata was not UTF-8: {e}"))?;
    parse_metadata(&json, dir)
}

/// Parse a `cargo metadata` JSON document, resolving the package whose
/// directory is `manifest_dir`. Kept separate from `inspect` so the JSON
/// reader is testable with a synthetic document.
pub fn parse_metadata(json: &str, manifest_dir: &Path) -> Result<Project, String> {
    let doc = Json::parse(json)?;
    let root = doc
        .get("workspace_root")
        .and_then(Json::as_str)
        .map(PathBuf::from)
        .ok_or("cargo metadata: no workspace_root")?;
    let target_dir = doc
        .get("target_directory")
        .and_then(Json::as_str)
        .map(PathBuf::from)
        .ok_or("cargo metadata: no target_directory")?;
    let packages = doc
        .get("packages")
        .and_then(Json::as_array)
        .ok_or("cargo metadata: no packages")?;

    // Match the package whose manifest lives in `manifest_dir`; fall back to
    // the first package (a single-package project) when paths are not
    // resolvable. `cargo metadata` reports absolute manifest paths.
    let wanted = std::fs::canonicalize(manifest_dir).ok();
    let pkg = packages
        .iter()
        .find(|p| {
            p.get("manifest_path")
                .and_then(Json::as_str)
                .and_then(|m| Path::new(m).parent())
                .and_then(|d| std::fs::canonicalize(d).ok())
                .zip(wanted.as_ref())
                .is_some_and(|(a, b)| a == *b)
        })
        .or_else(|| packages.first())
        .ok_or("cargo metadata: no packages")?;

    let package_name = pkg
        .get("name")
        .and_then(Json::as_str)
        .ok_or("cargo metadata: package has no name")?
        .to_string();
    let features = pkg
        .get("features")
        .and_then(Json::as_object)
        .map(|m| m.iter().map(|(k, _)| k.clone()).collect())
        .unwrap_or_default();
    Ok(Project {
        root,
        target_dir,
        package_name,
        features,
    })
}

/// The default features `scripts/build.sh` derives: use a `<platform>`
/// feature if the package has one, else `real`, else none. Explicit
/// `--features` from the user wins (handled by the packer).
pub fn default_features(features: &[String], platform: &str) -> Vec<String> {
    if features.iter().any(|f| f == platform) {
        vec![platform.to_string()]
    } else if features.iter().any(|f| f == "real") {
        vec!["real".to_string()]
    } else {
        Vec::new()
    }
}

/// Require every SDK precondition to be satisfied; return the joined hints of
/// any that are missing. This is the "detect and require, never auto-install"
/// rule, centralized so every packer shares it.
pub fn require_sdks(sdks: &[SdkPrecondition]) -> Result<(), String> {
    let missing: Vec<&SdkPrecondition> = sdks.iter().filter(|s| !s.is_satisfied()).collect();
    if missing.is_empty() {
        Ok(())
    } else {
        let hints = missing
            .iter()
            .map(|s| s.hint())
            .collect::<Vec<_>>()
            .join("; ");
        Err(hints)
    }
}

/// Require SDKs, but treat a missing one as a `blocked` tier (exit 0) rather
/// than a hard error: SDKs cannot be auto-installed, so their absence is a
/// blocked build, and the setup hint becomes the result note. The orchestrator
/// prints the `TIER=blocked` line and exits 0.
pub fn require_sdks_blocked(sdks: &[SdkPrecondition]) -> Result<(), PackResult> {
    match require_sdks(sdks) {
        Ok(()) => Ok(()),
        Err(hint) => Err(PackResult {
            tier: Tier::Blocked,
            artifact: None,
            note: Some(hint),
        }),
    }
}

/// Append `--features <comma-list>` to a cargo argv when the caller derived any.
/// Keeps the `<platform>` → `real` → none feature derivation in effect for the
/// platform build (the orchestrator computes it; the cargo commands apply it).
pub fn append_features(argv: &mut Vec<String>, features: &[String]) {
    if !features.is_empty() {
        argv.push("--features".to_string());
        argv.push(features.join(","));
    }
}

/// Run `program` with `args` inside `dir`, returning an error that includes the
/// argv and exit status on failure. Shared by every packer that shells out.
pub fn run(program: &str, args: &[&str], dir: &Path) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("cannot run `{} {}`: {e}", program, args.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{} {}` failed ({status})",
            program,
            args.join(" ")
        ))
    }
}

/// Find the most-recently-modified file under `dir` whose name ends with
/// `.{ext}` (ext given without the dot). Returns `None` if nothing matches.
/// Used to locate the artifact a native build dropped, regardless of where the
/// toolchain placed it within the build tree.
pub fn newest_artifact(dir: &Path, ext: &str) -> Option<PathBuf> {
    let needle = format!(".{ext}");
    let Ok(walker) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = newest_artifact(&path, ext) {
                push_if_newer(&mut best, &found);
            }
            continue;
        }
        let Some(name) = path
            .file_name()
            .map(|n| n.to_string_lossy().ends_with(&needle))
        else {
            continue;
        };
        if !name {
            continue;
        }
        push_if_newer(&mut best, &path);
    }
    best.map(|(_, p)| p)
}

fn push_if_newer(best: &mut Option<(std::time::SystemTime, PathBuf)>, path: &Path) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    let Ok(mtime) = meta.modified() else {
        return;
    };
    if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
        *best = Some((mtime, path.to_path_buf()));
    }
}

/// A deliberately small JSON value — only what cargo metadata's fields need.
enum Json {
    Null,
    Bool,
    Number,
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }
    fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(o) => Some(o),
            _ => None,
        }
    }
    fn get(&self, key: &str) -> Option<&Json> {
        self.as_object()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    fn parse(input: &str) -> Result<Json, String> {
        let mut r = Reader {
            bytes: input.as_bytes(),
            pos: 0,
        };
        let value = r.value()?;
        r.ws();
        if r.pos != r.bytes.len() {
            return Err("cargo metadata: trailing data after JSON value".into());
        }
        Ok(value)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn expect(&mut self, b: u8) -> Result<(), String> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "cargo metadata: expected '{}' at {}",
                b as char, self.pos
            ))
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Json::Bool)
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Json::Bool)
            }
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Json::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            other => Err(format!(
                "cargo metadata: unexpected byte {other:?} at {}",
                self.pos
            )),
        }
    }
    fn literal(&mut self, word: &[u8]) -> Result<(), String> {
        if self.bytes[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(())
        } else {
            Err(format!("cargo metadata: bad literal at {}", self.pos))
        }
    }
    fn number(&mut self) -> Result<Json, String> {
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.pos += 1;
        }
        Ok(Json::Number)
    }
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let b = self.peek().ok_or("cargo metadata: unterminated string")?;
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self.peek().ok_or("cargo metadata: unterminated escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            // Paths in metadata are plain ASCII; a surrogate
                            // pair would not survive this minimal reader, so
                            // decode a single \uXXXX code point.
                            let hex = self
                                .bytes
                                .get(self.pos..self.pos + 4)
                                .ok_or("cargo metadata: short \\u escape")?;
                            self.pos += 4;
                            let s = std::str::from_utf8(hex)
                                .map_err(|_| "cargo metadata: bad \\u escape")?;
                            let cp = u32::from_str_radix(s, 16)
                                .map_err(|_| "cargo metadata: bad \\u hex")?;
                            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err("cargo metadata: unknown escape".into()),
                    }
                }
                _ => {
                    // Consume a full UTF-8 sequence started at `pos - 1`.
                    let start = self.pos - 1;
                    let len = utf8_len(b);
                    let end = start + len;
                    let s = std::str::from_utf8(
                        self.bytes
                            .get(start..end)
                            .ok_or("cargo metadata: bad UTF-8 in string")?,
                    )
                    .map_err(|_| "cargo metadata: bad UTF-8 in string")?;
                    out.push_str(s);
                    self.pos = end;
                }
            }
        }
    }
    fn object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            self.expect(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(entries));
                }
                _ => {
                    return Err(format!(
                        "cargo metadata: expected ',' or '}}' at {}",
                        self.pos
                    ));
                }
            }
        }
    }
    fn array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => {
                    return Err(format!(
                        "cargo metadata: expected ',' or ']' at {}",
                        self.pos
                    ));
                }
            }
        }
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const META: &str = r#"{
      "workspace_root": "/r",
      "target_directory": "/r/target",
      "packages": [
        {"name": "demo", "manifest_path": "/r/Cargo.toml",
         "features": {"real": [], "android": []}}
      ]
    }"#;

    #[test]
    fn parse_metadata_extracts_project_fields() {
        let p = parse_metadata(META, Path::new("/r")).unwrap();
        assert_eq!(p.root, Path::new("/r"));
        assert_eq!(p.target_dir, Path::new("/r/target"));
        assert_eq!(p.package_name, "demo");
        assert!(p.features.contains(&"real".to_string()));
        assert!(p.features.contains(&"android".to_string()));
    }

    #[test]
    fn parse_metadata_rejects_broken_json() {
        assert!(parse_metadata("not json", Path::new("/r")).is_err());
    }

    #[test]
    fn default_feature_prefers_platform_then_real_then_none() {
        let f = vec!["real".to_string(), "android".to_string()];
        assert_eq!(default_features(&f, "android"), vec!["android"]);
        assert_eq!(default_features(&f, "ios"), vec!["real"]);
        assert_eq!(default_features(&[], "android"), Vec::<String>::new());
    }

    #[test]
    fn requiring_no_sdks_always_succeeds() {
        assert!(require_sdks(&[]).is_ok());
    }

    #[test]
    fn append_features_adds_a_comma_list_only_when_present() {
        let mut argv = vec!["build".to_string()];
        append_features(&mut argv, &[]);
        assert_eq!(argv, vec!["build"]);

        argv = vec!["build".to_string()];
        append_features(&mut argv, &["android".to_string(), "real".to_string()]);
        assert_eq!(argv, vec!["build", "--features", "android,real"]);
    }

    #[test]
    fn requiring_a_missing_sdk_fails_with_its_hint() {
        // `__definitely_not_a_real_target__` as a stand-in is not an SDK; use
        // a real precondition whose satisfaction we control via a fake command
        // is overkill — pin the hint-to-error wiring with an always-absent one.
        let err = require_sdks(&[SdkPrecondition::Hvigor]).unwrap_err();
        if !SdkPrecondition::Hvigor.is_satisfied() {
            assert!(err.contains("DEVECO_SDK_HOME"), "{err}");
        }
    }

    #[test]
    fn newest_artifact_finds_a_matching_file_recursively() {
        let dir = std::env::temp_dir().join(format!("floom-cli-artifact-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/b")).unwrap();
        std::fs::write(dir.join("a/b/app.apk"), b"x").unwrap();
        std::fs::write(dir.join("a/notes.txt"), b"x").unwrap();
        let found = newest_artifact(&dir, "apk").unwrap();
        assert_eq!(found.file_name().unwrap(), "app.apk");
        assert!(newest_artifact(&dir, "ipa").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
