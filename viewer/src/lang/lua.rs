//! Lua: `s=[[...]]load(s:gsub("%s",""))()`.
//!
//! The long string `[[...]]` holds the code broken anywhere by whitespace,
//! `gsub("%s","")` strips it back out and `load` compiles the result. The body
//! in `lua.lua` is written so that it means the same with every whitespace
//! character removed, and it never contains `]]` or `--`. `layout.rs` mirrors
//! what it does when run. Needs Lua 5.3 or later for `//` and integers.

use super::{Frame, Language, layout};
use crate::fractal::Params;

const TEMPLATE: &str = include_str!("lua.lua");
const HEAD: &str = "s=[[";
const TAIL: &str = "]]load(s:gsub(\"%s\",\"\"))()";

pub struct Lua;

impl Language for Lua {
    fn name(&self) -> &'static str {
        "Lua"
    }
    fn extension(&self) -> &'static str {
        "lua"
    }
    fn tool(&self) -> &'static str {
        "lua"
    }
    fn render(&self, p: &Params) -> Result<Frame, String> {
        // The tail has to fit on one row.
        if p.w < TAIL.len() {
            return Err(format!("frame narrower than {} columns", TAIL.len()));
        }
        layout::render(p, layout::fill(TEMPLATE, p), HEAD, TAIL)
    }
}
