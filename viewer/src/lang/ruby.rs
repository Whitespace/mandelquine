//! Ruby: `eval(s=%w(...)*"")`.
//!
//! Inside `%w(...)` whitespace only separates words and the words are joined
//! with "", so the code can be broken at any character and poured into the
//! fractal's shape. `ruby.rb` is the program body (whitespace stripped at
//! build time); this file mirrors what that body does when run, so the frame
//! shown in the viewer is byte-for-byte what the program prints.

use super::{Frame, Glyph, Language};
use crate::fractal::{Params, escape_grid};

const TEMPLATE: &str = include_str!("ruby.rb");
const HEAD: &str = "eval(s=%w(";
const TAIL: &str = ")*\"\")";

pub struct Ruby;

/// Shortest round-trip literal. `{:e}` gives e.g. `-7.5e-1`, `0e0`, which
/// Ruby parses to the same f64.
pub fn literal(x: f64) -> String {
    format!("{x:e}")
}

fn body(p: &Params) -> String {
    let stripped: String = TEMPLATE.split_whitespace().collect();
    stripped
        .replacen("W", &p.w.to_string(), 1)
        .replacen("H", &p.h.to_string(), 1)
        .replacen("M", &p.m.to_string(), 1)
        .replacen("X0", &literal(p.x0), 1)
        .replacen("Y0", &literal(p.y0), 1)
        .replacen("SC", &literal(p.sc), 1)
}

impl Language for Ruby {
    fn name(&self) -> &'static str {
        "Ruby"
    }
    fn extension(&self) -> &'static str {
        "rb"
    }
    fn interpreter(&self) -> &'static [&'static str] {
        &["ruby"]
    }
    fn min_cells(&self, p: &Params) -> usize {
        body(p).len() + HEAD.len() + TAIL.len() + p.w
    }

    fn render(&self, p: &Params) -> Result<Frame, String> {
        let (w, h) = (p.w, p.h);
        let n = w * h;
        if w < 20 {
            return Err("frame narrower than 20 columns".into());
        }
        if n < self.min_cells(p) {
            return Err("frame too small for the program".into());
        }
        let t = body(p);
        let tb = t.as_bytes();
        let v = escape_grid(p);

        // City-block distance from each interior cell to the nearest cell
        // that isn't interior, with a vertical step costing 2 because cells
        // are about twice as tall as wide. Two chamfer passes, as in ruby.rb.
        let far = n as i64;
        let mut d: Vec<i64> = v.iter().map(|&z| if z < 8 * p.m { 0 } else { far }).collect();
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

        // o=(0...w*h).sort_by{|j|[-v[j],d[j],j]}: interior first, and within
        // the interior the cells nearest its edge, so the code draws the
        // outline and the middle stays empty.
        let mut o: Vec<usize> = (0..n).collect();
        o.sort_unstable_by_key(|&j| (std::cmp::Reverse(v[j]), d[j], j));

        let l = t.len() + HEAD.len() + TAIL.len();
        let mut pp = l;

        let mut g = vec![false; n];
        let (mut a, mut b) = (0usize, 0usize);
        let lay = |q: usize, g: &mut Vec<bool>, a: &mut usize, b: &mut usize| -> usize {
            g.iter_mut().for_each(|c| *c = false);
            for &j in o.iter().take(q) {
                g[j] = true;
            }
            let first = g.iter().position(|&c| c).unwrap();
            *a = first.min(first / w * w + w - 10);
            for d in 0..10 {
                g[*a + d] = true;
            }
            let last = g.iter().rposition(|&c| c).unwrap();
            *b = last.max(last / w * w + 4);
            for d in 0..5 {
                g[*b - d] = true;
            }
            g.iter().filter(|&&c| c).count()
        };
        let mut c = lay(pp, &mut g, &mut a, &mut b);
        while c > l {
            if pp <= 1 {
                return Err("could not place the program".into());
            }
            pp -= 1;
            c = lay(pp, &mut g, &mut a, &mut b);
        }
        for &j in &o {
            if c < l && !g[j] && j > a && j < b {
                g[j] = true;
                c += 1;
            }
        }
        if c != l {
            return Err("could not find room for the program".into());
        }

        let e: Vec<u8> = [HEAD.as_bytes(), tb, TAIL.as_bytes()].concat();
        let mut glyphs = vec![Glyph::Empty; n];
        let mut k = 0;
        for (j, cell) in g.iter().enumerate() {
            if *cell {
                glyphs[j] = Glyph::Code(e[k] as char);
                k += 1;
            }
        }

        let mut text = String::with_capacity(n + h);
        for y in 0..h {
            let row: String = glyphs[y * w..(y + 1) * w]
                .iter()
                .map(|gl| match gl {
                    Glyph::Code(c) => *c,
                    Glyph::Empty => ' ',
                })
                .collect();
            text.push_str(row.trim_end());
            text.push('\n');
        }

        Ok(Frame { params: *p, text, glyphs, value: v, code_len: l })
    }
}
