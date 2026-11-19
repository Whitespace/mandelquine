//! Quine back ends. Each one turns a view into program text that, when run,
//! prints exactly that text again.
//!
//! Adding a language: write a `Language` impl in `lang/<name>.rs` and add it
//! to `all()`. Nothing else in the viewer names a language; the UI, export,
//! the V check and the tests all go through this trait. If the wrapper is
//! whitespace-insensitive with an unbreakable head and tail (as Ruby's is),
//! `layout::fill` and `layout::render` do the work, and the body file must
//! mirror `layout.rs` exactly. Otherwise `render` may lay the frame out
//! however the language needs.

pub mod c;
pub mod javascript;
pub mod layout;
pub mod lua;
pub mod perl;
pub mod ruby;
pub mod scheme;

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::fractal::Params;

/// What a cell of the frame holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Empty,
    /// Part of the executable program.
    Code(char),
}

pub struct Frame {
    pub params: Params,
    /// The program, exactly as it prints itself (trailing newline included).
    pub text: String,
    pub glyphs: Vec<Glyph>,
    /// Escape-time value per cell, see `fractal::escape_grid`.
    pub value: Vec<u32>,
    /// Characters in the program, whitespace excluded.
    pub code_len: usize,
}

pub trait Language: Send + Sync {
    fn name(&self) -> &'static str;
    /// File extension for exported and verified frames, without the dot.
    fn extension(&self) -> &'static str;
    /// The interpreter or compiler the V check needs, e.g. "ruby" or "cc".
    /// Shown in the status bar; tests are skipped when it isn't on PATH.
    fn tool(&self) -> &'static str;
    /// Lay the program out for the view `p`. `p.outside` says whether the
    /// code goes inside or outside the set; the program must carry it.
    fn render(&self, p: &Params) -> Result<Frame, String>;
    /// Fewest cells (w*h) a frame needs for this program to fit; below it
    /// `render` may fail with "frame too small" and the tests skip that size.
    fn min_cells(&self) -> usize {
        0
    }
    /// Run the frame saved at `src` and return what it printed. `scratch` is
    /// an empty directory for build products, removed afterwards. The default
    /// runs `tool src`; a compiled language overrides this to build into
    /// `scratch` first and then run the binary, returning the compiler's
    /// output instead if the build fails.
    fn run(&self, src: &Path, scratch: &Path) -> io::Result<Output> {
        let _ = scratch;
        Command::new(self.tool()).arg(src).output()
    }
}

pub fn all() -> &'static [&'static dyn Language] {
    &[
        &ruby::Ruby,
        &perl::Perl,
        &javascript::JavaScript,
        &lua::Lua,
        &scheme::Scheme,
        &c::C,
    ]
}

/// Whether the language's tool can be started at all.
#[cfg(test)]
pub fn available(lang: &dyn Language) -> bool {
    Command::new(lang.tool()).arg("--version").output().is_ok()
}

/// Save `text` as a program, run it, and check it prints itself.
pub fn verify(lang: &dyn Language, text: &str) -> Result<(), String> {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "mandelview-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let scratch = dir.join("build");
        std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
        let src = dir.join(format!("mandelquine.{}", lang.extension()));
        std::fs::write(&src, text).map_err(|e| e.to_string())?;
        let out = lang
            .run(&src, &scratch)
            .map_err(|e| format!("couldn't run {}: {e}", lang.tool()))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr)
                .lines()
                .next()
                .unwrap_or("failed")
                .to_string());
        }
        if out.stdout != text.as_bytes() {
            return Err("output differs from the frame".into());
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}
