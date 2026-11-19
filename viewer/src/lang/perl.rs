//! Perl: `eval($s=q(...)=~y/!-~//cdr)`.
//!
//! `q(...)` holds the code as a string broken anywhere by whitespace, and
//! `y/!-~//cdr` returns it with everything outside printable ASCII deleted,
//! which is the whitespace. `perl.pl` is the program body; `layout.rs`
//! mirrors what it does when run. Needs Perl 5.14 for the `r` flag.

use super::{Frame, Language, layout};
use crate::fractal::Params;

const TEMPLATE: &str = include_str!("perl.pl");
const HEAD: &str = "eval($s=q(";
const TAIL: &str = ")=~y/!-~//cdr)";

pub struct Perl;

impl Language for Perl {
    fn name(&self) -> &'static str {
        "Perl"
    }
    fn extension(&self) -> &'static str {
        "pl"
    }
    fn interpreter(&self) -> &'static [&'static str] {
        &["perl"]
    }
    fn render(&self, p: &Params) -> Result<Frame, String> {
        layout::render(p, layout::fill(TEMPLATE, p), HEAD, TAIL)
    }
}
