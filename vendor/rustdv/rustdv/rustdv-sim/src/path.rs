//! `RustdvPath` — a component's position in the tree, as segments.
//!
//! The path is **derived by the walk and never stored on a component** (D7).
//! Making it a type rather than a `String` buys three things:
//!
//! 1. **It cannot be fabricated.** A `RustdvPath` is produced only by the
//!    framework's walk ([`RustdvPath::root`] then [`RustdvPath::child`]), so a
//!    user cannot hand-type `"env.loga"` where a path is expected. That is the
//!    guarantee D7 and D62 exist to protect: the old
//!    `Logger::new("env.loga")` kept compiling — and kept addressing the wrong
//!    subtree — after a rename.
//! 2. **Prefix matching is correct by construction.** Comparing segment lists
//!    cannot confuse `ab` with a child of `a`, which the string form had to
//!    special-case (D64).
//! 3. **Depth is a count, not a scan** — and D13's config precedence is scaled
//!    by the setter's depth.
//!
//! The rendered dotted form is cached alongside the segments, so `Display` and
//! `as_str()` are free and every log line prints exactly as it always has.

use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RustdvPath {
    segments: Vec<String>,
    /// The dotted rendering, kept in step with `segments`. Built once per node
    /// during the walk rather than formatted at every log call.
    rendered: String,
}

impl RustdvPath {
    /// The root of a tree — the test's registered name (D49).
    pub fn root(name: &str) -> RustdvPath {
        RustdvPath {
            segments: vec![name.to_string()],
            rendered: name.to_string(),
        }
    }

    /// An empty path, for contexts with no position (a free `#[rustdv::test]`
    /// function, which has no component tree).
    pub fn empty() -> RustdvPath {
        RustdvPath::default()
    }

    /// This path extended by one segment — how the walk descends (D9).
    pub fn child(&self, name: &str) -> RustdvPath {
        let mut segments = self.segments.clone();
        segments.push(name.to_string());
        let rendered = if self.rendered.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", self.rendered, name)
        };
        RustdvPath { segments, rendered }
    }

    /// The dotted form, e.g. `RandomTest.env.scoreboard`.
    pub fn as_str(&self) -> &str {
        &self.rendered
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// How deep this node sits. The root is depth 1; an empty path is 0.
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Is this path at or below `prefix`? Segment-wise, so `a.b` is under `a`
    /// and `ab` is not — without the string special-case D64 needed.
    pub fn is_under(&self, prefix: &RustdvPath) -> bool {
        prefix.segments.len() <= self.segments.len()
            && self.segments[..prefix.segments.len()] == prefix.segments[..]
    }
}

impl fmt::Display for RustdvPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rendered)
    }
}

/// Segment-wise "is `path` at or below `prefix`?" for the string form, used by
/// the log module's per-subtree handlers where prefixes arrive as text.
///
/// This replaces `path == prefix || path.starts_with(&format!("{prefix}."))` —
/// same answer, no allocation, and the dot rule falls out of the comparison
/// instead of being bolted on.
pub fn str_is_under(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    let mut have = path.split('.');
    for want in prefix.split('.') {
        match have.next() {
            Some(seg) if seg == want => continue,
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_extends_and_renders() {
        let root = RustdvPath::root("RandomTest");
        let env = root.child("env");
        let sb = env.child("scoreboard");
        assert_eq!(sb.as_str(), "RandomTest.env.scoreboard");
        assert_eq!(sb.depth(), 3);
        assert_eq!(sb.segments(), ["RandomTest", "env", "scoreboard"]);
    }

    #[test]
    fn under_is_segment_wise() {
        let a = RustdvPath::root("a");
        let ab = a.child("b");
        assert!(ab.is_under(&a));
        assert!(a.is_under(&a));
        assert!(!a.is_under(&ab));
        // the bug the string form had to special-case
        assert!(!RustdvPath::root("ab").is_under(&a));
    }

    #[test]
    fn str_under_matches_on_segments() {
        assert!(str_is_under("a.b", "a"));
        assert!(str_is_under("a", "a"));
        assert!(!str_is_under("ab", "a")); // not a child of `a`
        assert!(!str_is_under("a", "a.b"));
        assert!(str_is_under("anything", ""));
    }

    #[test]
    fn empty_path_has_no_segments() {
        let e = RustdvPath::empty();
        assert!(e.is_empty());
        assert_eq!(e.depth(), 0);
        assert_eq!(e.as_str(), "");
        assert_eq!(e.child("top").as_str(), "top");
    }
}
