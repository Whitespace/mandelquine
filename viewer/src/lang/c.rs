//! C: `#define Q(...)char*s=#__VA_ARGS__;__VA_ARGS__` then `Q( body )`.
//!
//! The macro hands the body to the compiler twice: once as code and once
//! stringified into `s`. Stringifying collapses every run of whitespace
//! between tokens to one space, so the program recovers its own source by
//! dropping the spaces from `s`. Unlike Ruby's `%w`, C can't take whitespace
//! inside a token, so the layout here is token-aware: a token only starts in
//! a run of cells long enough to hold it, and the cells it can't use are left
//! blank. The `#define` line and the one string literal in the body can't
//! take a bare newline either, so their rows end in a `\` line splice.
//!
//! `c.c` is the body (whitespace stripped at build time); `render` mirrors
//! what it does when run, so the frame is byte-for-byte what it prints.

use std::io;
use std::path::Path;
use std::process::{Command, Output};

use super::{Frame, Glyph, Language, layout};
use crate::fractal::{Params, escape_grid};

const TEMPLATE: &str = include_str!("c.c");
/// The directive, as the body's `t` literal spells it. `/**/` stands in for
/// the space `define Q` needs, since the body can't carry whitespace.
const HEAD: &str = "#define/**/Q(...)char*s=#__VA_ARGS__;__VA_ARGS__";
/// The body's arrays hold 2^18 cells, and the rank key packs a cell index
/// into 18 bits.
const MAX_CELLS: usize = 1 << 18;

pub struct C;

/// Letters, digits, `_`, `.` and `-` form one class, everything else the
/// other. A break (whitespace) is allowed only between classes, which never
/// splits a token as long as the body avoids `-=` and `->`.
fn class(z: u8) -> bool {
    z.is_ascii_alphanumeric() || z == b'_' || z == b'.' || z == b'-'
}

impl Language for C {
    fn name(&self) -> &'static str {
        "C"
    }
    fn extension(&self) -> &'static str {
        "c"
    }
    fn tool(&self) -> &'static str {
        "cc"
    }
    /// The program is about 1.7k characters, plus the cells lost to tokens
    /// that don't fit the end of a run.
    fn min_cells(&self) -> usize {
        2000
    }

    fn run(&self, src: &Path, scratch: &Path) -> io::Result<Output> {
        let bin = scratch.join("q");
        let out = Command::new("cc")
            .arg("-O2")
            .arg("-o")
            .arg(&bin)
            .arg(src)
            .output()?;
        if !out.status.success() {
            return Ok(out);
        }
        Command::new(&bin).output()
    }

    fn render(&self, p: &Params) -> Result<Frame, String> {
        let (w, h) = (p.w, p.h);
        let n = w * h;
        if w < 20 {
            return Err("frame narrower than 20 columns".into());
        }
        if n > MAX_CELLS {
            return Err("frame too large for the C program".into());
        }
        let body = layout::fill(TEMPLATE, p);

        // e = HEAD + "Q(" + body + ")". a0 is where `Q(` starts, (b0, r0]
        // the inside of the body's string literal.
        let e: Vec<u8> = [HEAD.as_bytes(), b"Q(", body.as_bytes(), b")"].concat();
        let z = e.len();
        if n < z + w {
            return Err("frame too small for the program".into());
        }
        let a0 = HEAD.len();
        let b0 = e.iter().position(|&c| c == b'"').unwrap();
        let r0 = e.iter().rposition(|&c| c == b'"').unwrap();
        // No break between `Q` and `(` in the directive, or Q would be an
        // object-like macro. The body hard-codes that index as 12.
        let brk =
            |k: usize| k == 0 || (k > b0 && k <= r0) || (k != 12 && class(e[k - 1]) != class(e[k]));
        let splice = |k: usize| (k > 0 && k < a0) || (k > b0 && k <= r0);

        let v = escape_grid(p);

        // Same chamfer distance as layout.rs.
        let far = n as i64;
        let mut d: Vec<i64> = v
            .iter()
            .map(|&s| if s < 8 * p.m { 0 } else { far })
            .collect();
        let (wi, ni) = (w as i64, n as i64);
        for k in [1i64, -1] {
            for i in 0..ni {
                let j = (ni - 1) * (1 - k) / 2 + k * i;
                if (j - k).div_euclid(wi) == j / wi && d[j as usize] > d[(j - k) as usize] + 1 {
                    d[j as usize] = d[(j - k) as usize] + 1;
                }
                let nb = j - k * wi;
                if nb >= 0 && nb < ni && d[j as usize] > d[nb as usize] + 2 {
                    d[j as usize] = d[nb as usize] + 2;
                }
            }
        }

        // Same ranking as layout.rs. The body packs the key into one double,
        // ((score + 8m) * 2^18 + d) * 2^18 + j, and sorts that.
        let mut o: Vec<usize> = (0..n).collect();
        let (m8, u) = (8 * p.m as i64, p.outside as i64);
        o.sort_unstable_by_key(|&j| (-(v[j] as i64) * (1 - v[j] as i64 / m8 * u), d[j], j));

        // rank[j] is the cell's place in that order; the best l cells are
        // those with rank < l.
        let mut rank = vec![0usize; n];
        for (i, &j) in o.iter().enumerate() {
            rank[j] = i;
        }

        // Pour the program into the best l cells, token by token. Returns
        // whether all of it fit. This is the body's J().
        let mut out = vec![b' '; n];
        let mut run = vec![0usize; n];
        let mut pour = |l: usize, out: &mut Vec<u8>| -> bool {
            // Length of the run of chosen cells starting at each cell.
            for j in (0..n).rev() {
                run[j] = if rank[j] >= l {
                    0
                } else if j % w < w - 1 {
                    run[j + 1] + 1
                } else {
                    1
                };
            }
            let mut k = 0;
            let mut last_row = 0usize;
            for y in 0..h {
                let a = (0..w).rev().find(|&x| rank[y * w + x] < l);
                for x in 0..w {
                    let j = y * w + x;
                    out[j] = b' ';
                    // The directive must end on its own line.
                    if rank[j] >= l || k >= z || (k == a0 && y == last_row) {
                        continue;
                    }
                    if brk(k) {
                        // A token starts here: place it only if the run
                        // holds it, plus a `\` if the row has to splice
                        // straight after it.
                        let mut t = 1;
                        while k + t < z && !brk(k + t) {
                            t += 1;
                        }
                        let ends_row = j + run[j] - 1 == y * w + a.unwrap();
                        if Some(x) == a && splice(k) {
                            out[j] = b'\\';
                        } else if t + (splice(k + t) && ends_row) as usize <= run[j] {
                            out[j] = e[k];
                            k += 1;
                            last_row = y;
                        }
                    } else {
                        out[j] = e[k];
                        k += 1;
                        last_row = y;
                    }
                }
                // A row with no cells inside a splice region still needs
                // its `\`.
                if a.is_none() && splice(k) {
                    out[y * w] = b'\\';
                }
            }
            k == z
        };

        // Smallest-ish l that fits: gallop up from z by 1, 2, 4, ... cells
        // (capped at n), then bisect between the last miss and the first fit.
        if !pour(n, &mut out) {
            return Err("could not place the program".into());
        }
        let (mut lo, mut hi, mut step) = (z - 1, z, 1);
        while !pour(hi, &mut out) {
            lo = hi;
            hi = (lo + step).min(n);
            step *= 2;
        }
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if pour(mid, &mut out) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        pour(hi, &mut out);

        let glyphs: Vec<Glyph> = out
            .iter()
            .map(|&c| {
                if c == b' ' {
                    Glyph::Empty
                } else {
                    Glyph::Code(c as char)
                }
            })
            .collect();
        let mut text = String::with_capacity(n + h);
        for y in 0..h {
            let row = std::str::from_utf8(&out[y * w..(y + 1) * w]).unwrap();
            text.push_str(row.trim_end());
            text.push('\n');
        }
        let code_len = out.iter().filter(|&&c| c != b' ').count();
        Ok(Frame {
            params: *p,
            text,
            glyphs,
            value: v,
            code_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::{available, verify};

    fn params(w: usize, h: usize, x0: f64, y0: f64, sc: f64, m: u32, outside: bool) -> Params {
        let lit = |x: f64| -> f64 { layout::literal(x).parse().unwrap() };
        Params {
            w,
            h,
            m,
            x0: lit(x0),
            y0: lit(y0),
            sc: lit(sc),
            outside,
        }
    }

    /// Views away from the whole set, where the code has to wrap around
    /// thin filaments and isolated islands.
    #[test]
    fn c_frames_are_fixed_points() {
        if !available(&C) {
            eprintln!("cc not found, skipping");
            return;
        }
        let views = [
            (110, 48, -0.75, 0.0, 3.09e-2, 100),
            (150, 50, -0.1528, 1.0397, 1.2e-3, 400),
            (120, 40, -1.7685, 0.0, 6e-4, 450),
            (100, 45, 0.285, 0.0115, 3e-5, 900),
            (90, 40, 5.0, 5.0, 1e-2, 100),
        ];
        for (w, h, x0, y0, sc, m) in views {
            for outside in [false, true] {
                let p = params(w, h, x0, y0, sc, m, outside);
                let frame = C.render(&p).expect("layout");
                if let Err(e) = verify(&C, &frame.text) {
                    let _ = std::fs::write(std::env::temp_dir().join("c-fail.c"), &frame.text);
                    panic!("{w}x{h} at {x0},{y0} sc={sc} outside={outside}: {e}");
                }
            }
        }
    }
}
