//! Quine back ends. Each one turns a view into program text that, when run by
//! that language's interpreter, prints exactly that text again.

pub mod ruby;

use crate::fractal::Params;

/// What a cell of the frame holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Empty,
    /// Part of the executable program.
    Code(char),
    /// Padding inside a trailing comment, used when the set's interior has
    /// more cells than the program has characters.
    Pad(char),
}

pub struct Frame {
    pub params: Params,
    /// The program, exactly as it prints itself (trailing newline included).
    pub text: String,
    pub glyphs: Vec<Glyph>,
    /// Escape-time value per cell, see `fractal::escape_grid`.
    pub value: Vec<u32>,
    /// Characters of executable code, excluding padding.
    pub code_len: usize,
    pub pad_len: usize,
}

pub trait Language: Send + Sync {
    fn name(&self) -> &'static str;
    fn extension(&self) -> &'static str;
    /// Command that runs a program file and prints to stdout, e.g. ["ruby"].
    fn interpreter(&self) -> &'static [&'static str];
    /// Smallest frame the layout can work with.
    fn min_cells(&self, p: &Params) -> usize;
    fn render(&self, p: &Params) -> Result<Frame, String>;
}

pub fn all() -> Vec<Box<dyn Language>> {
    vec![Box::new(ruby::Ruby)]
}
