//! Scheme (GNU Guile 3): `(define(s)(string-filter char-set:graphic"...")) (eval-string(s))`.
//!
//! The body lives in a string, so the layout may break it anywhere, and
//! `(s)` returns it with the whitespace filtered out. Scheme code needs
//! spaces between atoms, which that filter would also delete, so the string
//! holds a short whitespace-free bootstrap followed by `#|...|#`, a block
//! comment containing `scheme.scm` with each space written as `_`. The
//! bootstrap cuts the comment out of `(s)`, turns `_` back into spaces and
//! evaluates the result. `scheme.scm` mirrors `layout.rs`.

use std::io;
use std::path::Path;
use std::process::{Command, Output};

use super::{Frame, Language, layout};
use crate::fractal::Params;

const TEMPLATE: &str = include_str!("scheme.scm");
const HEAD: &str = "(define(s)(string-filter char-set:graphic\"";
const TAIL: &str = "\"))(eval-string(s))";

pub struct Scheme;

/// `scheme.scm` with runs of whitespace collapsed to one space, spaces next
/// to parentheses dropped, and the remaining spaces written as `_`.
fn encode(src: &str) -> String {
    let mut s = src.split_whitespace().collect::<Vec<_>>().join(" ");
    for (a, b) in [(" (", "("), ("( ", "("), (" )", ")"), (") ", ")")] {
        while s.contains(a) {
            s = s.replace(a, b);
        }
    }
    s.replace(' ', "_")
}

/// The bootstrap: evaluate `(s)` from index `b` (just past `#|`) up to the
/// closing `|#`, with every `_` turned back into a space. It can't contain
/// whitespace, backslashes or `"`, hence `(+(+)95)` for the `_` character.
/// `b` is its own length plus 2, so it is found by trying each digit count.
fn boot() -> String {
    let f = |b: usize| {
        format!(
            "(eval-string(string-join(string-split(substring(s){b}(-(string-length(s))2))(integer->char(+(+)95)))))"
        )
    };
    (1..1000)
        .map(f)
        .find(|t| t.contains(&format!("(s){}(", t.len() + 2)))
        .expect("bootstrap offset")
}

fn body() -> String {
    format!("{}#|{}|#", boot(), encode(TEMPLATE))
}

impl Language for Scheme {
    fn name(&self) -> &'static str {
        "Scheme"
    }
    fn extension(&self) -> &'static str {
        "scm"
    }
    fn tool(&self) -> &'static str {
        "guile"
    }
    fn render(&self, p: &Params) -> Result<Frame, String> {
        // The head is pinned to a single row.
        if p.w < HEAD.len() + 2 {
            return Err(format!("frame narrower than {} columns", HEAD.len() + 2));
        }
        layout::render(p, layout::fill(&body(), p), HEAD, TAIL)
    }
    /// The program is about 1,850 characters at most, against Ruby's 800,
    /// because every space Scheme needs costs a character.
    fn min_cells(&self) -> usize {
        2100
    }
    fn run(&self, src: &Path, scratch: &Path) -> io::Result<Output> {
        let _ = scratch;
        Command::new(self.tool())
            .arg("--no-auto-compile")
            .arg(src)
            .output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_no_forbidden_characters() {
        let b = body();
        for c in [' ', '\n', '\t', '"', '\\'] {
            assert!(!b.contains(c), "body contains {c:?}");
        }
        assert!(!TEMPLATE.contains('_'), "scheme.scm may not use `_`");
        // The comment must hold exactly one `|#`, at the end.
        assert_eq!(b.matches("|#").count(), 1);
        assert!(b.ends_with("|#"));
        // Only the placeholders may be uppercase, so `fill` finds them.
        let upper: String = b.chars().filter(|c| c.is_ascii_uppercase()).collect();
        assert_eq!(upper, "WHMXYSCU");
        // scheme.scm hard-codes the wrapper's lengths.
        assert_eq!((HEAD.len(), TAIL.len()), (42, 19));
    }
}
