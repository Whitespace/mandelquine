//! Cosine gradients (a + b*cos(2pi(c*t + d))), cycled over time.

use ratatui::style::Color;
use std::f64::consts::TAU;

pub struct Palette {
    pub name: &'static str,
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    d: [f64; 3],
}

pub const PALETTES: &[Palette] = &[
    Palette {
        name: "aurora",
        a: [0.5, 0.5, 0.5],
        b: [0.5, 0.5, 0.5],
        c: [1.0, 1.0, 1.0],
        d: [0.0, 0.33, 0.67],
    },
    Palette {
        name: "ember",
        a: [0.5, 0.4, 0.3],
        b: [0.5, 0.4, 0.3],
        c: [1.0, 1.0, 1.0],
        d: [0.0, 0.1, 0.2],
    },
    Palette {
        name: "ocean",
        a: [0.3, 0.5, 0.6],
        b: [0.3, 0.4, 0.4],
        c: [1.0, 1.0, 1.0],
        d: [0.5, 0.6, 0.7],
    },
    Palette {
        name: "acid",
        a: [0.5, 0.5, 0.5],
        b: [0.5, 0.5, 0.5],
        c: [2.0, 1.0, 0.0],
        d: [0.5, 0.2, 0.25],
    },
    Palette {
        name: "phosphor",
        a: [0.15, 0.55, 0.2],
        b: [0.15, 0.45, 0.2],
        c: [1.0, 1.0, 1.0],
        d: [0.0, 0.0, 0.0],
    },
    Palette {
        name: "mono",
        a: [0.5, 0.5, 0.5],
        b: [0.5, 0.5, 0.5],
        c: [1.0, 1.0, 1.0],
        d: [0.0, 0.0, 0.0],
    },
];

impl Palette {
    pub fn rgb(&self, t: f64) -> [f64; 3] {
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] =
                (self.a[i] + self.b[i] * (TAU * (self.c[i] * t + self.d[i])).cos()).clamp(0.0, 1.0);
        }
        out
    }

    /// Gradient colour at `t`, scaled by `k` and mixed toward white by `white`.
    pub fn color(&self, t: f64, k: f64, white: f64) -> Color {
        let c = self.rgb(t);
        let f = |x: f64| {
            ((x * k * (1.0 - white) + white) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Color::Rgb(f(c[0]), f(c[1]), f(c[2]))
    }
}
