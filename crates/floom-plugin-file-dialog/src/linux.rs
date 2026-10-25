//! Linux file dialogs via `zenity`, falling back to `kdialog`.
//!
//! The same shape `open-url` uses for `xdg-open`: spawn the desktop's own
//! tool and read its answer. `zenity` is the GNOME-world dialog tool and the
//! one most distributions ship; `kdialog` is KDE's. Trying zenity first and
//! kdialog when zenity is not installed covers both worlds without linking a
//! widget toolkit into the process.
//!
//! # What the exit codes mean
//!
//! Both tools exit `0` with the chosen path on stdout, and `1` when the user
//! cancels — which is this crate's `Ok(None)`, the user's answer. Any other
//! exit is the tool failing, and is reported with its code. A missing binary
//! is reported as exactly that (a minimal container has no dialog tool),
//! not as a broken capability.

use super::Options;
use std::path::PathBuf;
use std::process::Command;

pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String> {
    run_dialog(
        options,
        |o| zenity_args(o, "--file-selection", false),
        |o| kdialog_args(o, "--getopenfilename"),
    )
}

pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String> {
    run_dialog(
        options,
        |o| zenity_args(o, "--file-selection", true),
        |o| kdialog_args(o, "--getsavefilename"),
    )
}

pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String> {
    run_dialog(
        options,
        |o| zenity_args(o, "--file-selection --directory", false),
        |o| kdialog_args(o, "--getexistingdirectory"),
    )
}

/// Drive zenity (or kdialog) and interpret the outcome.
///
/// One function serves all three picks because the tools' contracts are
/// identical apart from the mode flags: path on stdout, `0` chosen, `1`
/// cancelled.
fn run_dialog(
    options: &Options,
    zenity: impl Fn(&Options) -> Vec<String>,
    kdialog: impl Fn(&Options) -> Vec<String>,
) -> Result<Option<PathBuf>, String> {
    let output = Command::new("zenity")
        .args(zenity(options))
        .output()
        .or_else(|_| {
            // zenity is not installed; KDE machines have kdialog instead.
            Command::new("kdialog").args(kdialog(options)).output()
        })
        .map_err(|e| {
            format!(
                "no dialog tool could be run — tried `zenity` and `kdialog` ({e}). \
                 A minimal system may have neither installed"
            )
        })?;

    match output.status.code() {
        Some(0) => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                // Chosen, but nothing on stdout: the tool's way of saying the
                // selection had no path (a cancelled native selection, a
                // non-filesystem entry). The honest mapping is "no answer".
                Ok(None)
            } else {
                Ok(Some(PathBuf::from(path)))
            }
        }
        Some(1) => Ok(None), // the user cancelled — the user's answer
        Some(code) => Err(format!(
            "the dialog tool exited with {code} — \
             stderr: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        None => Err("the dialog tool was killed by a signal".into()),
    }
}

/// `zenity --file-selection [--save] [--directory] [--title=T] [--filename=N]`.
fn zenity_args(options: &Options, mode: &str, save: bool) -> Vec<String> {
    let mut args: Vec<String> = mode.split_whitespace().map(String::from).collect();
    if save {
        args.push("--save".into());
    }
    if let Some(title) = &options.title {
        args.push(format!("--title={title}"));
    }
    if let Some(name) = &options.suggested_name {
        args.push(format!("--filename={name}"));
    }
    args
}

/// `kdialog --getopenfilename|--getsavefilename|--getexistingdirectory [start]`.
fn kdialog_args(options: &Options, mode: &str) -> Vec<String> {
    let mut args = vec![mode.to_string(), ".".into()];
    if let Some(title) = &options.title {
        args.push("--title".into());
        args.push(title.clone());
    }
    args
}
