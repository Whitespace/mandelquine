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

        // o=(0...w*h).sort_by{|j|[-v[j],j]}
        let mut o: Vec<usize> = (0..n).collect();
        o.sort_unstable_by_key(|&j| (std::cmp::Reverse(v[j]), j));

        let l = t.len() + HEAD.len() + TAIL.len();
        let ic = v.iter().filter(|&&z| z >= 8 * p.m).count();
        let mut pp = l.max(ic);

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
        while pp > ic && c > l {
            pp -= 1;
            c = lay(pp, &mut g, &mut a, &mut b);
        }
        for &j in &o {
            if c < l && !g[j] && j > a && j < b {
                g[j] = true;
                c += 1;
            }
        }
        if c < l {
            return Err("could not find room for the program".into());
        }

        // Padding comment: '#', then a prefix of the code (repeated) cut where
        // its parens can still be closed, then ';' filler and the closers.
        let mut fx = Vec::<u8>::new();
        if c > l {
            let room = (c - l - 1) as i64;
            let (mut fq, mut fd) = (0i64, 0i64);
            for k in 0..tb.len() * c {
                let ch = tb[k % tb.len()];
                let mut dd = 81 - 2 * ch as i64;
                if dd.abs() > 1 {
                    dd = 0;
                }
                if fq + fd + 1 + dd > room {
                    break;
                }
                fq += 1;
                fd += dd;
            }
            fx.push(b'#');
            fx.extend((0..fq as usize).map(|k| tb[k % tb.len()]));
            fx.extend(std::iter::repeat_n(b';', (room - fq - fd) as usize));
            fx.extend(std::iter::repeat_n(b')', fd as usize));
        }

        let code_head = HEAD.len() + t.len();
        let e: Vec<u8> = [HEAD.as_bytes(), tb, &fx, TAIL.as_bytes()].concat();
        let mut glyphs = vec![Glyph::Empty; n];
        let mut k = 0;
        for (j, cell) in g.iter().enumerate() {
            if *cell && k < e.len() {
                let ch = e[k] as char;
                let is_pad = k >= code_head && k < code_head + fx.len();
                glyphs[j] = if is_pad { Glyph::Pad(ch) } else { Glyph::Code(ch) };
                k += 1;
            }
        }

        let mut text = String::with_capacity(n + h);
        for y in 0..h {
            let row: String = glyphs[y * w..(y + 1) * w]
                .iter()
                .map(|gl| match gl {
                    Glyph::Code(c) | Glyph::Pad(c) => *c,
                    Glyph::Empty => ' ',
                })
                .collect();
            text.push_str(row.trim_end());
            text.push('\n');
        }

        Ok(Frame { params: *p, text, glyphs, value: v, code_len: l, pad_len: fx.len() })
    }
}
