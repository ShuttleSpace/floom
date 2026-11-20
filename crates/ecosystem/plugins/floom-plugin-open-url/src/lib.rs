//! Open a URL with the application the OS routes its scheme to.
//!
//! This is `url_launcher`'s question — "show this `https://` in a browser,
//! this `mailto:` in a mail app, this `tel:` in a dialer" — and it is the
//! catalog's A2 (`docs/plugins.md`): the implementation half of the
//! handoff doc's C4 decision, which deleted the `OPEN_URL` capability bit
//! because a bit with no operation behind it is a promise nothing keeps.
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # What `Ok` means — and what it does not
//!
//! `Ok(())` claims one thing: **the OS accepted the request.** It does not
//! claim a window opened, a browser launched, or the user saw anything. Every
//! arm here hands a URL to the platform's routing and returns what the routing
//! said — `startActivity` returning, `openURL:`'s `BOOL`, `ShellExecuteW`'s
//! `HINSTANCE`, `xdg-open`'s exit status. Whether the handler then shows
//! anything is outside this crate's knowledge, and pretending otherwise (an
//! `Ok` that waited for a window) is not possible over these APIs at all.
//!
//! This is the "true launch" category from the catalog's verification matrix:
//! the action is the answer, and the caller who needs more must watch for
//! focus or use a deeper integration.
//!
//! # Every arm is a real read
//!
//! Like `floom-plugin-paths`, there is no host-inert arm on the desktops:
//! macOS routes through `NSWorkspace`, Linux through `xdg-open`, Windows
//! through `ShellExecuteW`, and those are the platform's own mechanisms — a
//! host is the source of truth here, not a stand-in for a phone. Only targets
//! with no such mechanism (a terminal UI, say) get the refusal `Err`.
//!
//! # What this crate deliberately does not do
//!
//! `url_launcher` has a launch *mode* — external application, in-app web
//! view, platform default. The in-app mode is a webview, which is a different
//! catalog entry (B1, `floom-plugin-webview`, provider ABI and all); this
//! crate is the external mode only, and says so rather than shipping an
//! in-app fallback that renders nothing. It also does not answer
//! `canLaunchUrl` — "does some app handle this scheme" is a query with its own
//! platform story (intent resolution on Android, Launch Services on macOS),
//! and half of it behind the same function would blur what `Ok` means.

/// Open `url` with the application the OS routes its scheme to.
///
/// The URL must carry a scheme (`https://…`, `mailto:…`): the OS routes by
/// scheme, and a bare string has nothing to route. Callers wanting "open this
/// file" want `floom-plugin-paths` plus their own viewer, not a `file://` URL
/// — Android has refused those cross-app since API 24, and the arm reports
/// the platform's refusal verbatim rather than pre-empting it.
pub fn open_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("open_url: the URL is empty".into());
    }
    if !has_scheme(url) {
        return Err(format!(
            "open_url: `{url}` has no scheme — the OS routes by scheme \
             (pass `https://…`, `mailto:…`); a bare string has nothing to route"
        ));
    }
    #[cfg(target_os = "android")]
    {
        android::open_url(url)
    }
    #[cfg(target_os = "ios")]
    {
        ios::open_url(url)
    }
    #[cfg(target_os = "macos")]
    {
        macos::open_url(url)
    }
    #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
    {
        linux::open_url(url)
    }
    #[cfg(target_os = "windows")]
    {
        windows::open_url(url)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::open_url(url)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        Err(format!(
            "open_url: this target ({}) has no URL routing \
             — Android, iOS, macOS, Linux and Windows do",
            std::env::consts::OS
        ))
    }
}

/// Whether the string starts with an RFC 3986 scheme: `ALPHA *( ALPHA / DIGIT
/// / "+" / "-" / "." ) ":"`.
///
/// This is deliberately *not* full URL parsing — the OS does that, better and
/// with locale awareness. The check exists only to fail before dispatch, so a
/// typo reaches the caller as this crate's reason instead of as five
/// different platform errors for the same mistake.
fn has_scheme(url: &str) -> bool {
    let Some(at) = url.find(':') else {
        return false;
    };
    let scheme = &url[..at];
    let mut chars = scheme.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_string_has_nothing_to_route() {
        // Fails before dispatch, so this is side-effect-free on every host:
        // no arm is reached, no browser opens. The same is true of the empty
        // case below — which is the whole reason validation lives here and
        // not in the arms.
        assert!(open_url("example.com/page").is_err());
        assert!(open_url("").is_err());
        assert!(open_url("://missing-scheme").is_err());
        assert!(open_url("1nvalid-scheme://x").is_err());
    }

    #[test]
    fn the_scheme_check_is_rfc_3986() {
        assert!(has_scheme("https://example.com"));
        assert!(has_scheme("mailto:someone@example.com"));
        assert!(has_scheme("tel:+1-555-0100"));
        assert!(has_scheme("custom+app.v2://deep/link"));
        // A scheme must start with a letter and contain only the RFC's
        // characters; anything else is a bare string.
        assert!(!has_scheme("example.com"));
        assert!(!has_scheme("https")); // a colon is what makes it a scheme
        assert!(!has_scheme("1http://x"));
        assert!(!has_scheme("ht tp://x"));
    }

    // There is deliberately no happy-path test here. A successful
    // `open_url("https://…")` on a desktop opens a real browser window — a
    // side effect a test suite must not have. The verification path for the
    // real arms is the `floom-capabilities` gallery's action button, which a
    // human presses and watches; see the gallery's actions module.
}
