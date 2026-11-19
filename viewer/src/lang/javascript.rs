//! JavaScript (Node): ``eval(s=`...`.replace(/[^!-~]/g,""))``.
//!
//! The template literal holds the code broken anywhere by whitespace, and the
//! `replace` deletes everything outside printable ASCII, which is the
//! whitespace. The body has no backticks, backslashes or `${`, so the template
//! literal passes it through untouched; it writes those characters with
//! `String.fromCharCode`. It assigns undeclared globals, so the frame must run
//! as a sloppy-mode script (a plain `.js` file outside an ES module package).
//! `javascript.js` is the program body; `layout.rs` mirrors what it does when
//! run.

use super::{Frame, Language, layout};
use crate::fractal::Params;

const TEMPLATE: &str = include_str!("javascript.js");
const HEAD: &str = "eval(s=`";
const TAIL: &str = "`.replace(/[^!-~]/g,\"\"))";

pub struct JavaScript;

impl Language for JavaScript {
    fn name(&self) -> &'static str {
        "JavaScript"
    }
    fn extension(&self) -> &'static str {
        "js"
    }
    fn tool(&self) -> &'static str {
        "node"
    }
    fn render(&self, p: &Params) -> Result<Frame, String> {
        layout::render(p, layout::fill(TEMPLATE, p), HEAD, TAIL)
    }
}
