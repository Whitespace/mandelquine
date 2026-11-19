//! The layout every language shares: score the cells, rank them, pour the
//! program into the best ones. Each template (`ruby.rb`, `perl.pl`) does
//! exactly this when run, so the two must be changed together.

use super::{Frame, Glyph};
use crate::fractal::{Params, escape_grid};

/// Shortest round-trip literal. `{:e}` gives e.g. `-7.5e-1`, `0e0`, which
/// Ruby and Perl both parse to the same f64.
pub fn literal(x: f64) -> String {
    format!("{x:e}")
}

/// Strip a template's whitespace and substitute the view's parameters for
/// its W, H, M, X0, Y0, SC and U placeholders.
pub fn fill(template: &str, p: &Params) -> String {
    let stripped: String = template.split_whitespace().collect();
    stripped
        .replacen("W", &p.w.to_string(), 1)
        .replacen("H", &p.h.to_string(), 1)
        .replacen("M", &p.m.to_string(), 1)
        .replacen("X0", &literal(p.x0), 1)
        .replacen("Y0", &literal(p.y0), 1)
        .replacen("SC", &literal(p.sc), 1)
        .replacen("U", if p.outside { "1" } else { "0" }, 1)
}

pub fn min_cells(body: &str, head: &str, tail: &str, p: &Params) -> usize {
    body.len() + head.len() + tail.len() + p.w
}

/// Lay `head + body + tail` out for the view `p`. `head` and `tail` are the
/// parts of the wrapper that can't contain whitespace.
pub fn render(p: &Params, t: String, head: &str, tail: &str) -> Result<Frame, String> {
    let (w, h) = (p.w, p.h);
    let n = w * h;
    if w < 20 {
        return Err("frame narrower than 20 columns".into());
    }
    if n < min_cells(&t, head, tail, p) {
        return Err("frame too small for the program".into());
    }
    let tb = t.as_bytes();
    let v = escape_grid(p);

    // City-block distance from each interior cell to the nearest cell
    // that isn't interior, with a vertical step costing 2 because cells
    // are about twice as tall as wide. Two chamfer passes, as in every template.
    let far = n as i64;
    let mut d: Vec<i64> = v
        .iter()
        .map(|&z| if z < 8 * p.m { 0 } else { far })
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

    // Rank by [-v*(1-v/(8m)*u), d, j].
    // With u=0 the interior ranks first, and within it the cells nearest
    // its edge, so the code draws the outline and the middle stays empty.
    // With u=1 the interior's score becomes 0, which ranks it last, so the
    // code sits in the exterior cells closest to the set.
    let mut o: Vec<usize> = (0..n).collect();
    let (m8, u) = (8 * p.m as i64, p.outside as i64);
    o.sort_unstable_by_key(|&j| (-(v[j] as i64) * (1 - v[j] as i64 / m8 * u), d[j], j));

    let l = t.len() + head.len() + tail.len();

    // Take the l best cells, then pin the wrapper's two unbreakable ends:
    // `head` on the first cells and `tail` on the last, each kept on one
    // row (g = 2). That can add up to head+tail cells, so drop the same
    // number of the worst-ranked unpinned ones.
    let (hl, tl) = (head.len(), tail.len());
    let mut g = vec![0u8; n];
    for &j in o.iter().take(l) {
        g[j] = 1;
    }
    let first = g.iter().position(|&c| c == 1).unwrap();
    let a = first.min(first / w * w + w - hl);
    for i in 0..hl {
        g[a + i] = 2;
    }
    let last = g
        .iter()
        .rposition(|&c| c == 1)
        .ok_or("frame too small for the program")?;
    let b = last.max(last / w * w + tl - 1);
    if b < a + hl + tl - 1 {
        return Err("frame too small for the program".into());
    }
    for i in 0..tl {
        g[b - i] = 2;
    }
    let mut c = g.iter().filter(|&&c| c > 0).count();
    for &j in o.iter().take(l).rev() {
        if c > l && g[j] == 1 {
            g[j] = 0;
            c -= 1;
        }
    }
    if c != l {
        return Err("could not place the program".into());
    }

    let e: Vec<u8> = [head.as_bytes(), tb, tail.as_bytes()].concat();
    let mut glyphs = vec![Glyph::Empty; n];
    let mut k = 0;
    for (j, cell) in g.iter().enumerate() {
        if *cell > 0 {
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

    Ok(Frame {
        params: *p,
        text,
        glyphs,
        value: v,
        code_len: l,
    })
}
