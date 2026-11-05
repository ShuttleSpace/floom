// SPDX-License-Identifier: MIT
//! The deep-link boundary: the one stringly-typed edge in navigation.
//!
//! Everything above this module is typed ([`Route`](crate::Route)); everything
//! below it is the platform handing the app a URL. Parsing happens here and
//! nowhere else, so query decoding rules live in exactly one place.

/// A parsed deep link: path + query, with the scheme/host discarded (they only
/// say *which app* got the link, and by the time the app sees this it knows).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Url {
    /// The path, normalised to start with `/`.
    pub path: String,
    query: Vec<(String, String)>,
}

/// A URL that could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlError(&'static str);

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for UrlError {}

/// Parses `scheme://host/path?query` into a [`Url`].
///
/// Tolerant by design: the scheme, host and a leading `?` are all optional
/// (deep links arrive as `app://host/profile/42`, `profile/42` or
/// `/profile/42`), while percent-escapes in path and query are decoded.
/// An empty input is the only hard error.
pub fn parse(input: &str) -> Result<Url, UrlError> {
    let mut rest = input.trim();
    if rest.is_empty() {
        return Err(UrlError("empty url"));
    }

    // scheme://
    if let Some(idx) = rest.find("://") {
        rest = &rest[idx + 3..];
    }

    // host — the segment before the first `/` or `?`.
    let after_host = match rest.find(['/', '?']) {
        Some(idx) => &rest[idx..],
        None => "",
    };

    // path?query
    let (path, query) = match after_host.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (after_host, None),
    };

    let mut path = decode_component(path);
    if path.is_empty() || !path.starts_with('/') {
        path.insert(0, '/');
    }

    let mut query_pairs = Vec::new();
    if let Some(query) = query {
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            query_pairs.push((decode_component(key), decode_component(value)));
        }
    }

    Ok(Url {
        path,
        query: query_pairs,
    })
}

impl Url {
    /// The first value for `name`, if present.
    pub fn query(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// All values for `name` in order (repeated parameters).
    pub fn query_all(&self, name: &str) -> impl Iterator<Item = &str> {
        self.query
            .iter()
            .filter(move |(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The query as `(key, value)` pairs, in order.
    pub fn query_pairs(&self) -> &[(String, String)] {
        &self.query
    }
}

/// Decodes `%XX` escapes and `+` (as space, per form-encoding convention).
///
/// Invalid escapes decode as literal characters rather than failing: a deep
/// link that is *almost* right should still route somewhere sensible.
fn decode_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = bytes.get(index + 1..index + 3).and_then(|h| {
                    std::str::from_utf8(h)
                        .ok()
                        .and_then(|h| u8::from_str_radix(h, 16).ok())
                });
                match hex {
                    Some(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    None => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_and_bare_urls() {
        let full = parse("floom://app/profile/42?ref=push&tab=info").expect("valid");
        assert_eq!(full.path, "/profile/42");
        assert_eq!(full.query("ref"), Some("push"));
        assert_eq!(full.query("tab"), Some("info"));

        for bare in ["/profile/42", "profile/42", "?x=1"] {
            let url = parse(bare).expect("tolerant parsing");
            assert!(
                url.path.starts_with('/'),
                "{bare:?} normalises to a rooted path, got {:?}",
                url.path
            );
        }
    }

    #[test]
    fn query_helpers_cover_repeats_and_missing() {
        let url = parse("/search?q=one&q=two&flag").expect("valid");
        assert_eq!(url.query("q"), Some("one"));
        assert_eq!(url.query_all("q").collect::<Vec<_>>(), vec!["one", "two"]);
        assert_eq!(url.query("flag"), Some(""), "bare key maps to empty value");
        assert_eq!(url.query("nope"), None);
        assert_eq!(url.query_pairs().len(), 3);
    }

    #[test]
    fn percent_escapes_and_plus_decode() {
        let url = parse("/a%20b/c+d?name=Zo%C3%AB&bad=%ZZ").expect("valid");
        assert_eq!(url.path, "/a b/c d");
        assert_eq!(url.query("name"), Some("Zoë"));
        // Invalid escapes degrade to literals instead of failing.
        assert_eq!(url.query("bad"), Some("%ZZ"));
    }

    #[test]
    fn empty_input_is_the_only_hard_error() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }
}
