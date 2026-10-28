//! Linux: libsecret through `secret-tool`.
//!
//! The same shape `floom-plugin-open-url` uses for `xdg-open`: hand the work
//! to the desktop's own tool rather than re-implement its protocol. The item's
//! attributes are `service <store>` and `account <key>`, which is how the
//! libsecret CLI names a generic secret.
//!
//! `secret-tool` is line-oriented on both ends, so a value ending in a newline
//! cannot round-trip. `set` refuses one instead of storing a changed secret.

use std::io::Write;
use std::process::{Command, Output, Stdio};

const TOOL: &str = "secret-tool";

pub fn get(service: &str, key: &str) -> Result<Option<String>, String> {
    let output = run(&["lookup", "service", service, "account", key], None)?;
    if !output.status.success() {
        // `secret-tool` exits 1 when no item matches; anything else is a real
        // failure (no session bus, a locked keyring).
        if output.status.code() == Some(1) {
            return Ok(None);
        }
        return Err(failure("lookup", &output));
    }
    let mut value = String::from_utf8(output.stdout)
        .map_err(|_| "secure-storage: the stored value is not UTF-8".to_string())?;
    if value.ends_with('\n') {
        value.pop();
    }
    Ok(Some(value))
}

pub fn set(service: &str, key: &str, value: &str) -> Result<(), String> {
    if value.ends_with('\n') {
        return Err(format!(
            "secure-storage: the value under {key:?} ends with a newline, which \
             `secret-tool` cannot store without changing it"
        ));
    }
    let label = format!("--label={service}/{key}");
    let output = run(
        &["store", &label, "service", service, "account", key],
        Some(value.as_bytes()),
    )?;
    if output.status.success() {
        Ok(())
    } else {
        Err(failure("store", &output))
    }
}

pub fn remove(service: &str, key: &str) -> Result<(), String> {
    let output = run(&["clear", "service", service, "account", key], None)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(failure("clear", &output))
    }
}

fn run(args: &[&str], stdin: Option<&[u8]>) -> Result<Output, String> {
    let mut command = Command::new(TOOL);
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("secure-storage: `{TOOL}` is not on PATH; install libsecret-tools")
        } else {
            format!("secure-storage: cannot run `{TOOL}`: {e}")
        }
    })?;
    if let Some(bytes) = stdin {
        child
            .stdin
            .take()
            .ok_or_else(|| format!("secure-storage: `{TOOL}` has no stdin to write to"))?
            .write_all(bytes)
            .map_err(|e| format!("secure-storage: cannot write to `{TOOL}`: {e}"))?;
    }
    child
        .wait_with_output()
        .map_err(|e| format!("secure-storage: `{TOOL}` did not finish: {e}"))
}

fn failure(what: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("secure-storage: `{TOOL} {what}` failed ({})", output.status)
    } else {
        format!("secure-storage: `{TOOL} {what}` failed: {stderr}")
    }
}
