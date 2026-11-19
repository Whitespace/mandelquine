//! Escape-time grid shared by every language template.
//!
//! Each template recomputes this grid itself when the frame is run, so the
//! arithmetic here must match the template operation for operation: same
//! sample offsets, same evaluation order, plain f64 multiply/add with no fused
//! multiply-add. Keep it boring.

use rayon::prelude::*;

/// Parameters a frame is computed from. Every float is stored exactly as the
/// generated program will parse it back (see `View::literal`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub w: usize,
    pub h: usize,
    pub m: u32,
    pub x0: f64,
    pub y0: f64,
    pub sc: f64,
}

/// For each of the w*h cells, the sum over a 2x2 supersample of the escape
/// iteration (1..=m), or 2m for a sample that never escapes. A cell whose four
/// samples are all inside has the value 8m.
pub fn escape_grid(p: &Params) -> Vec<u32> {
    let Params { w, h, m, x0, y0, sc } = *p;
    (0..w * h)
        .into_par_iter()
        .map(|j| {
            let mut a = 0u32;
            for q in 0..4usize {
                let cr = x0 + ((((j % w) as f64 + (q % 2) as f64 * 0.5) + 0.25) - w as f64 * 0.5) * sc;
                let ci = y0
                    + ((((h as f64 * 0.5) - (j / w) as f64) - (q / 2) as f64 * 0.5) - 0.25) * 2.0 * sc;
                let (mut x, mut y) = (0.0f64, 0.0f64);
                let mut esc = None;
                for i in 1..=m {
                    let r = x * x - y * y + cr;
                    y = 2.0 * x * y + ci;
                    x = r;
                    if x * x + y * y > 4.0 {
                        esc = Some(i);
                        break;
                    }
                }
                a += esc.unwrap_or(m * 2);
            }
            a
        })
        .collect()
}
