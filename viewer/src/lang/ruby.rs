//! Ruby: `eval(s=%w(...)*"")`.
//!
//! Inside `%w(...)` whitespace only separates words and the words are joined
//! with "", so the code can be broken at any character and poured into the
//! fractal's shape. `ruby.rb` is the program body (whitespace stripped at
//! build time); `layout.rs` mirrors what it does when run.

use super::{Frame, Language, layout};
use crate::fractal::Params;

const TEMPLATE: &str = include_str!("ruby.rb");
const HEAD: &str = "eval(s=%w(";
const TAIL: &str = ")*\"\")";

pub struct Ruby;

impl Language for Ruby {
    fn name(&self) -> &'static str {
        "Ruby"
    }
    fn extension(&self) -> &'static str {
        "rb"
    }
    fn tool(&self) -> &'static str {
        "ruby"
    }
    fn render(&self, p: &Params) -> Result<Frame, String> {
        layout::render(p, layout::fill(TEMPLATE, p), HEAD, TAIL)
    }
}
