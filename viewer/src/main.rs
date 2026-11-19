//! mandelview: a Mandelbrot viewer whose every frame is a quine.
//!
//! The picture in the main panel is program text. Copy it (C), save it (X), or
//! check it (V): running it prints exactly the same text back.

mod fractal;
mod lang;
mod palette;

use std::io::Write as _;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Widget, Wrap,
};
use ratatui::{DefaultTerminal, Frame as TuiFrame};
use tachyonfx::{EffectManager, Interpolation, Motion, fx};

use fractal::Params;
use lang::{Frame, Glyph, Language};
use palette::PALETTES;

const ZOOM_STEP: f64 = 1.25;
const PAN_CELLS: f64 = 2.0;
const MIN_SC: f64 = 1e-13;
const MAX_SC: f64 = 0.25;
const BG: Color = Color::Rgb(8, 8, 14);
/// Code lying inside the set: dark grey on the near-black interior.
const INSIDE_FG: Color = Color::Rgb(105, 105, 105);

/// Round `x` to the fewest significant digits that still resolve `unit`, and
/// return the value the generated program will parse back.
fn tidy(x: f64, unit: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    let digits = ((x.abs() / unit).log10().ceil() as i32 + 3).clamp(1, 16) as usize;
    let s = format!("{:.*e}", digits, x);
    let v: f64 = s.parse().unwrap();
    lang::ruby::literal(v).parse().unwrap()
}

#[derive(Clone, Copy, PartialEq)]
struct View {
    cx: f64,
    cy: f64,
    sc: f64,
}

impl View {
    fn classic(w: usize, h: usize) -> Self {
        let sc = (3.4 / w as f64).max(2.45 / (2.0 * h as f64));
        let sc: f64 = format!("{sc:.2e}").parse().unwrap();
        View {
            cx: -0.75,
            cy: 0.0,
            sc,
        }
    }

    fn iterations(&self) -> u32 {
        let depth = (0.03 / self.sc).log2().max(0.0);
        (100.0 + 45.0 * depth).min(5000.0) as u32
    }

    fn params(&self, w: usize, h: usize, outside: bool) -> Params {
        Params {
            w,
            h,
            m: self.iterations(),
            x0: self.cx,
            y0: self.cy,
            sc: self.sc,
            outside,
        }
    }

    fn pan(&mut self, dx: f64, dy: f64) {
        self.cx = tidy(self.cx + dx, self.sc);
        self.cy = tidy(self.cy + dy, self.sc);
    }

    fn zoom(&mut self, factor: f64) -> bool {
        let sc = (self.sc * factor).clamp(MIN_SC, MAX_SC);
        let sc: f64 = format!("{sc:.2e}").parse().unwrap();
        let changed = sc != self.sc;
        self.sc = sc;
        self.cx = tidy(self.cx, sc);
        self.cy = tidy(self.cy, sc);
        changed
    }
}

#[derive(PartialEq)]
enum Mode {
    View,
    Languages(ListState),
    Help,
}

enum Verdict {
    Pending(Params),
    Done(Params, Result<(), String>),
}

struct App {
    langs: Vec<Box<dyn Language>>,
    lang: usize,
    view: Option<View>,
    frame: Option<Result<Frame, String>>,
    rendered: Option<(usize, View, usize, usize, bool)>,
    palette: usize,
    phase: f64,
    animate: bool,
    /// Code goes outside the set instead of inside it.
    outside: bool,
    /// Paint the escape-time gradient behind the frame.
    background: bool,
    mode: Mode,
    toast: Option<(String, Color, Instant)>,
    verdict: Option<Verdict>,
    verify_rx: Option<Receiver<(Params, Result<(), String>)>>,
    clipboard: Option<String>,
    fx: EffectManager<&'static str>,
    panel: Rect,
    started: bool,
    quit: bool,
}

impl App {
    fn new() -> Self {
        App {
            langs: lang::all(),
            lang: 0,
            view: None,
            frame: None,
            rendered: None,
            palette: 0,
            phase: 0.0,
            animate: false,
            outside: true,
            background: true,
            mode: Mode::View,
            toast: None,
            verdict: None,
            verify_rx: None,
            clipboard: None,
            fx: EffectManager::default(),
            panel: Rect::default(),
            started: false,
            quit: false,
        }
    }

    fn language(&self) -> &dyn Language {
        self.langs[self.lang].as_ref()
    }

    fn toast(&mut self, msg: impl Into<String>, color: Color) {
        self.toast = Some((msg.into(), color, Instant::now()));
    }

    fn frame(&self) -> Option<&Frame> {
        self.frame.as_ref().and_then(|f| f.as_ref().ok())
    }

    /// Recompute the frame if the panel size, view or language changed.
    fn refresh(&mut self, w: usize, h: usize) {
        let view = *self.view.get_or_insert_with(|| View::classic(w, h));
        let key = (self.lang, view, w, h, self.outside);
        if self.rendered == Some(key) {
            return;
        }
        self.rendered = Some(key);
        let p = view.params(w, h, self.outside);
        self.frame = Some(self.language().render(&p));
    }

    fn flash(&mut self, color: Color) {
        let e = fx::fade_from(color, color, (450, Interpolation::CubicOut)).with_area(self.panel);
        self.fx.add_unique_effect("flash", e);
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
            self.quit = true;
            return;
        }
        match &mut self.mode {
            Mode::Help => {
                self.mode = Mode::View;
                return;
            }
            Mode::Languages(state) => {
                let n = self.langs.len();
                match code {
                    KeyCode::Up | KeyCode::Char('w') => {
                        state.select(Some((state.selected().unwrap_or(0) + n - 1) % n))
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        state.select(Some((state.selected().unwrap_or(0) + 1) % n))
                    }
                    KeyCode::Enter => {
                        let pick = state.selected().unwrap_or(0);
                        self.mode = Mode::View;
                        if pick != self.lang {
                            self.lang = pick;
                            self.verdict = None;
                        }
                        let e = fx::coalesce((700, Interpolation::QuadOut)).with_area(self.panel);
                        self.fx.add_unique_effect("big", e);
                        let name = self.language().name();
                        self.toast(format!("frames are now {name} programs"), Color::LightCyan);
                    }
                    KeyCode::Esc | KeyCode::Char('l') => self.mode = Mode::View,
                    _ => {}
                }
                return;
            }
            Mode::View => {}
        }

        let Some(mut view) = self.view else { return };
        // PAN_CELLS cells per key press: a cell is sc wide and 2*sc tall.
        let (sx, sy) = (PAN_CELLS * view.sc, PAN_CELLS * 2.0 * view.sc);
        match code {
            KeyCode::Char('w') => {
                view.pan(0.0, sy);
            }
            KeyCode::Char('s') => {
                view.pan(0.0, -sy);
            }
            KeyCode::Char('a') => {
                view.pan(-sx, 0.0);
            }
            KeyCode::Char('d') => {
                view.pan(sx, 0.0);
            }
            KeyCode::Char('e') => {
                if !view.zoom(1.0 / ZOOM_STEP) {
                    self.toast("f64 precision limit: can't zoom further", Color::LightRed);
                }
            }
            KeyCode::Char('q') => {
                view.zoom(ZOOM_STEP);
            }
            KeyCode::Char('r') => {
                self.view = None;
                let e = fx::coalesce((900, Interpolation::QuadOut)).with_area(self.panel);
                self.fx.add_unique_effect("big", e);
                return;
            }
            KeyCode::Char('l') => {
                let mut st = ListState::default();
                st.select(Some(self.lang));
                self.mode = Mode::Languages(st);
            }
            KeyCode::Char('p') => {
                self.palette = (self.palette + 1) % PALETTES.len();
                let e =
                    fx::hsl_shift_fg([120.0, 0.0, 0.0], (400, Interpolation::SineOut)).reversed();
                self.fx.add_unique_effect("big", e.with_area(self.panel));
                self.toast(
                    format!("palette: {}", PALETTES[self.palette].name),
                    Color::Gray,
                );
            }
            KeyCode::Char(' ') => self.animate = !self.animate,
            KeyCode::Char('o') => {
                self.outside = !self.outside;
                let e = fx::coalesce((500, Interpolation::QuadOut)).with_area(self.panel);
                self.fx.add_unique_effect("big", e);
                self.toast(
                    if self.outside {
                        "code outside the set"
                    } else {
                        "code inside the set"
                    },
                    Color::Gray,
                );
            }
            KeyCode::Char('b') => {
                self.background = !self.background;
                self.toast(
                    if self.background {
                        "background gradient on"
                    } else {
                        "background gradient off"
                    },
                    Color::Gray,
                );
            }
            // These act on the frame, so make sure it reflects any moves
            // queued ahead of them in the same batch of input.
            KeyCode::Char('c') | KeyCode::Char('x') | KeyCode::Char('v') => {
                self.view = Some(view);
                if let Some((_, _, w, h, _)) = self.rendered {
                    self.refresh(w, h);
                }
                match code {
                    KeyCode::Char('c') => self.copy(),
                    KeyCode::Char('x') => self.export(),
                    _ => self.verify(),
                }
            }
            KeyCode::Char('?') | KeyCode::Char('h') => self.mode = Mode::Help,
            KeyCode::Esc => self.quit = true,
            _ => {}
        }
        self.view = Some(view);
    }

    fn copy(&mut self) {
        let Some(text) = self.frame().map(|f| f.text.clone()) else {
            return;
        };
        self.clipboard = Some(text);
        self.flash(Color::Rgb(90, 90, 120));
        self.toast("copied frame to clipboard (OSC 52)", Color::LightGreen);
    }

    fn export(&mut self) {
        let Some(f) = self.frame() else { return };
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = format!("mandelquine-{secs}.{}", self.language().extension());
        let text = f.text.clone();
        match std::fs::write(&path, text) {
            Ok(()) => {
                self.flash(Color::Rgb(90, 90, 120));
                self.toast(format!("wrote {path}"), Color::LightGreen);
            }
            Err(e) => self.toast(format!("write failed: {e}"), Color::LightRed),
        }
    }

    fn verify(&mut self) {
        let Some(f) = self.frame() else { return };
        let (params, text) = (f.params, f.text.clone());
        let cmd: Vec<String> = self
            .language()
            .interpreter()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let ext = self.language().extension();
        let path =
            std::env::temp_dir().join(format!("mandelview-verify-{}.{ext}", std::process::id()));
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let result = (|| {
                std::fs::write(&path, &text).map_err(|e| e.to_string())?;
                let out = std::process::Command::new(&cmd[0])
                    .args(&cmd[1..])
                    .arg(&path)
                    .output()
                    .map_err(|e| format!("couldn't run {}: {e}", cmd[0]))?;
                let _ = std::fs::remove_file(&path);
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
            let _ = tx.send((params, result));
        });
        self.verdict = Some(Verdict::Pending(params));
        self.verify_rx = Some(rx);
    }

    fn poll_verify(&mut self) {
        let Some(rx) = &self.verify_rx else { return };
        if let Ok((p, r)) = rx.try_recv() {
            self.verify_rx = None;
            let current = self.frame().map(|f| f.params) == Some(p);
            if current {
                self.flash(if r.is_ok() {
                    Color::Rgb(40, 140, 70)
                } else {
                    Color::Rgb(160, 40, 40)
                });
            }
            self.verdict = Some(Verdict::Done(p, r));
        }
    }
}

/// Paints the frame: code glyphs coloured by escape time, empty cells as a
/// dim glow, so copying the panel out of the terminal still yields the
/// program (backgrounds copy as spaces).
struct FrameWidget<'a> {
    background: bool,
    frame: &'a Frame,
    palette: &'a palette::Palette,
    phase: f64,
}

impl Widget for FrameWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let f = self.frame;
        let (w, h, m) = (f.params.w, f.params.h, f.params.m as f64);
        let ln_m = m.ln().max(1.0);
        for y in 0..h.min(area.height as usize) {
            for x in 0..w.min(area.width as usize) {
                let j = y * w + x;
                let v = f.value[j] as f64;
                let inside = f.value[j] >= 8 * f.params.m;
                let t = ((v / 4.0).max(1.0).ln() / ln_m).clamp(0.0, 1.0);
                let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + y as u16)) else {
                    continue;
                };
                let p = self.palette;
                let ph = self.phase;
                match f.glyphs[j] {
                    Glyph::Empty if inside || !self.background => {
                        cell.set_char(' ').set_fg(Color::Reset).set_bg(BG);
                    }
                    Glyph::Empty => {
                        let glow = 0.06 + 0.5 * t.powf(1.6);
                        cell.set_char(' ').set_fg(Color::Reset).set_bg(p.color(
                            t * 1.7 + ph,
                            glow,
                            0.0,
                        ));
                    }
                    Glyph::Code(c) if inside => {
                        cell.set_char(c).set_fg(INSIDE_FG).set_bg(BG);
                    }
                    Glyph::Code(c) if self.background => {
                        // Same background as the empty cells around it, so
                        // the gradient is unbroken. Text is white or black,
                        // whichever contrasts, tinted toward the complement
                        // of the background's hue.
                        let glow = 0.06 + 0.5 * t.powf(1.6);
                        let hue = p.rgb(t * 1.7 + ph);
                        let lum = (0.2126 * hue[0] + 0.7152 * hue[1] + 0.0722 * hue[2]) * glow;
                        let (base, tint) = if lum > 0.22 { (0.0, 0.12) } else { (1.0, 0.3) };
                        let ch = |i: usize| {
                            ((base * (1.0 - tint) + (1.0 - hue[i]) * tint) * 255.0).round() as u8
                        };
                        cell.set_char(c)
                            .set_fg(Color::Rgb(ch(0), ch(1), ch(2)))
                            .set_bg(p.color(t * 1.7 + ph, glow, 0.0))
                            .set_style(Style::new().add_modifier(Modifier::BOLD));
                    }
                    Glyph::Code(c) => {
                        cell.set_char(c)
                            .set_fg(p.color(t * 1.7 + ph, 1.0, 0.15))
                            .set_bg(BG)
                            .set_style(Style::new().add_modifier(Modifier::BOLD));
                    }
                }
            }
        }
    }
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

fn panel_layout(area: Rect) -> (Rect, Rect, Rect) {
    let [panel, status, keys] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    (panel, status, keys)
}

fn draw(app: &mut App, f: &mut TuiFrame, elapsed: Duration) {
    let area = f.area();
    f.buffer_mut().set_style(area, Style::new().bg(BG));
    let (panel, status, keys) = panel_layout(area);
    app.panel = panel;
    let pal = &PALETTES[app.palette];
    let accent = pal.color(app.phase, 1.0, 0.2);

    let lang = app.language().name();
    let title = Line::from(vec![
        Span::raw(" mandel").bold(),
        Span::styled("quine ", Style::new().fg(accent).bold()),
        Span::styled(format!("· {lang} "), Style::new().fg(Color::Gray)),
    ]);
    let hint = Line::from(Span::styled(
        " this picture is a program: run it and it prints itself ",
        Style::new().fg(Color::DarkGray).italic(),
    ))
    .alignment(Alignment::Right);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(pal.color(app.phase + 0.25, 0.7, 0.0)))
        .title(title)
        .title_bottom(hint);
    let inner = block.inner(panel);
    block.render(panel, f.buffer_mut());

    match &app.frame {
        Some(Ok(frame)) => FrameWidget {
            frame,
            palette: pal,
            phase: app.phase,
            background: app.background,
        }
        .render(inner, f.buffer_mut()),
        Some(Err(e)) => {
            let msg = Paragraph::new(format!(
                "can't lay out a frame here: {e}\nmake the terminal bigger, or zoom/pan"
            ))
            .alignment(Alignment::Center)
            .fg(Color::LightRed);
            f.render_widget(msg, centered(inner, 60, 3));
        }
        None => {}
    }

    // Status bar.
    let mut spans = vec![
        Span::styled(format!(" {lang} "), Style::new().fg(BG).bg(accent).bold()),
        Span::raw(" "),
    ];
    if let (Some(fr), Some(view)) = (app.frame(), app.view) {
        let zoom = View::classic(fr.params.w, fr.params.h).sc / view.sc;
        let sign = if view.cy < 0.0 { "-" } else { "+" };
        let dim = Style::new().fg(Color::DarkGray);
        spans.extend([
            Span::styled(
                format!("{:e} {sign} {:e}i", view.cx, view.cy.abs()),
                Style::new().fg(Color::White),
            ),
            Span::styled("  │  ", dim),
            Span::styled(format!("zoom {zoom:.3e}×"), Style::new().fg(Color::Gray)),
            Span::styled("  │  ", dim),
            Span::styled(
                format!("{} iter", fr.params.m),
                Style::new().fg(Color::Gray),
            ),
            Span::styled("  │  ", dim),
            Span::styled(
                format!(
                    "{} chars {}",
                    fr.code_len,
                    if fr.params.outside {
                        "outside"
                    } else {
                        "inside"
                    }
                ),
                Style::new().fg(Color::Gray),
            ),
            Span::styled("  │  ", dim),
        ]);
        let spin = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let verdict = match &app.verdict {
            Some(Verdict::Pending(p)) if *p == fr.params => {
                let i = (app.phase * 400.0) as usize % spin.len();
                Span::styled(
                    format!(
                        "{} verifying with {}",
                        spin[i],
                        app.language().interpreter()[0]
                    ),
                    Style::new().fg(Color::Yellow),
                )
            }
            Some(Verdict::Done(p, Ok(()))) if *p == fr.params => Span::styled(
                "✓ quine verified",
                Style::new().fg(Color::LightGreen).bold(),
            ),
            Some(Verdict::Done(p, Err(e))) if *p == fr.params => {
                Span::styled(format!("✗ {e}"), Style::new().fg(Color::LightRed).bold())
            }
            _ => Span::styled("V to verify", Style::new().fg(Color::DarkGray)),
        };
        spans.push(verdict);
    }
    if let Some((msg, color, at)) = &app.toast {
        if at.elapsed() < Duration::from_secs(3) {
            spans.push(Span::styled("  │  ", Style::new().fg(Color::DarkGray)));
            spans.push(Span::styled(msg.clone(), Style::new().fg(*color)));
        }
    }
    f.render_widget(Line::from(spans), status);

    let key = |k: &'static str, what: &'static str| {
        [
            Span::styled(
                format!(" {k} "),
                Style::new().fg(Color::Black).bg(Color::Gray),
            ),
            Span::styled(format!(" {what}  "), Style::new().fg(Color::Gray)),
        ]
    };
    let keyline: Vec<Span> = [
        key("WASD", "move"),
        key("Q/E", "zoom out/in"),
        key("L", "language"),
        key("O", "in/outside"),
        key("B", "background"),
        key("P", "palette"),
        key("␣", "animate"),
        key("C", "copy"),
        key("X", "export"),
        key("V", "verify"),
        key("R", "reset"),
        key("?", "help"),
        key("Esc", "quit"),
    ]
    .into_iter()
    .flatten()
    .collect();
    f.render_widget(Line::from(keyline), keys);

    if !app.started {
        app.started = true;
        app.fx
            .add_effect(fx::coalesce((1400, Interpolation::QuadOut)).with_area(inner));
        app.fx.add_effect(
            fx::sweep_in(
                Motion::LeftToRight,
                30,
                10,
                BG,
                (900, Interpolation::QuadOut),
            )
            .with_area(Rect::new(area.x, status.y, area.width, 2)),
        );
    }
    app.fx.process_effects(elapsed.into(), f.buffer_mut(), area);

    // Overlays are drawn after effects so they stay crisp.
    match &mut app.mode {
        Mode::Languages(state) => {
            let r = centered(area, 36, app.langs.len() as u16 + 2);
            f.render_widget(Clear, r);
            let items: Vec<ListItem> = app
                .langs
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let mark = if i == app.lang { "●" } else { " " };
                    ListItem::new(format!(" {mark} {}  (.{})", l.name(), l.extension()))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" quine language ")
                        .title_bottom(Line::from(" ↑↓/WS · Enter · Esc ").right_aligned())
                        .border_style(Style::new().fg(accent))
                        .bg(BG),
                )
                .highlight_style(Style::new().fg(BG).bg(accent).bold())
                .highlight_symbol("▶");
            f.render_stateful_widget(list, r, state);
        }
        Mode::Help => {
            let r = centered(area, 74, 18);
            f.render_widget(Clear, r);
            let text = vec![
                Line::from("Every frame in the panel is a complete program.".bold()),
                Line::from(""),
                Line::from("Copy it (C), or export it (X), and run it: it recomputes this exact"),
                Line::from("view of the Mandelbrot set, lays its own source out in the shape,"),
                Line::from("and prints the same text, byte for byte. V does that check for you."),
                Line::from(""),
                Line::from("The program has a fixed number of characters. They fill the set's"),
                Line::from("interior from its edge inward, so a large interior shows as an"),
                Line::from("outline; a small one overflows into the cells nearest the set."),
                Line::from(""),
                Line::from(vec![
                    "WASD ".bold(),
                    "pan   ".into(),
                    "Q/E ".bold(),
                    "zoom out/in   ".into(),
                    "R ".bold(),
                    "reset".into(),
                ]),
                Line::from(vec![
                    "O ".bold(),
                    "code inside/outside the set   ".into(),
                    "B ".bold(),
                    "background gradient".into(),
                ]),
                Line::from(vec![
                    "L ".bold(),
                    "language   ".into(),
                    "P ".bold(),
                    "palette   ".into(),
                    "Space ".bold(),
                    "colour cycling".into(),
                ]),
                Line::from(vec![
                    "C ".bold(),
                    "copy (OSC 52)   ".into(),
                    "X ".bold(),
                    "save to ./   ".into(),
                    "V ".bold(),
                    "verify".into(),
                ]),
                Line::from(""),
                Line::from("any key to close".dark_gray().italic()),
            ];
            let p = Paragraph::new(text).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" how it works ")
                    .border_style(Style::new().fg(accent))
                    .bg(BG),
            );
            f.render_widget(p, r);
        }
        Mode::View => {}
    }
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn run(term: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();
    let mut last = Instant::now();
    while !app.quit {
        let size = term.size()?;
        let (panel, _, _) = panel_layout(Rect::new(0, 0, size.width, size.height));
        let inner = Block::bordered().inner(panel);
        app.refresh(inner.width.max(1) as usize, inner.height.max(1) as usize);
        app.poll_verify();

        let now = Instant::now();
        let elapsed = now - last;
        last = now;
        if app.animate {
            app.phase = (app.phase + elapsed.as_secs_f64() * 0.04) % 1.0;
        }
        term.draw(|f| draw(&mut app, f, elapsed))?;

        if let Some(text) = app.clipboard.take() {
            let mut out = std::io::stdout();
            write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes()))?;
            out.flush()?;
        }

        // Wait up to a frame for input, then take everything that is queued
        // so a held key never lags behind the redraw.
        let mut wait = Duration::from_millis(33);
        while event::poll(wait)? {
            wait = Duration::ZERO;
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    let code = match k.code {
                        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                        c => c,
                    };
                    app.handle_key(code, k.modifiers);
                }
            }
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    if let Some(arg) = std::env::args().nth(1) {
        return cli(&arg);
    }
    ratatui::run(run)
}

/// `mandelview --print [cols rows]` writes the opening frame to stdout, for
/// scripts and tests.
fn cli(arg: &str) -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    match arg {
        "--print" => {
            let w = args.first().and_then(|s| s.parse().ok()).unwrap_or(110);
            let h = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(48);
            let view = View::classic(w, h);
            match lang::ruby::Ruby.render(&view.params(w, h, true)) {
                Ok(f) => print!("{}", f.text),
                Err(e) => eprintln!("{e}"),
            }
            Ok(())
        }
        _ => {
            eprintln!("usage: mandelview [--print [cols rows]]");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_values() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    /// The layout must succeed for any reasonable panel size and view,
    /// including blank and fully interior ones.
    #[test]
    fn layout_succeeds_across_sizes_and_views() {
        let centres = [
            (-0.75, 0.0),
            (5.0, 5.0),
            (-0.2, 0.0),
            (-1.75, 0.0),
            (0.3, 0.5),
        ];
        for w in (60..=240).step_by(7) {
            for h in [20, 33, 48, 70] {
                for (cx, cy) in centres {
                    for sc in [0.03, 0.004, 1e-5] {
                        for outside in [false, true] {
                            let view = View { cx, cy, sc };
                            let r = lang::ruby::Ruby.render(&view.params(w, h, outside));
                            assert!(
                                r.is_ok(),
                                "{w}x{h} at {cx},{cy} sc={sc} outside={outside}: {:?}",
                                r.err()
                            );
                        }
                    }
                }
            }
        }
    }

    /// Runs each frame through ruby and checks it prints itself. Skipped when
    /// ruby isn't on PATH.
    #[test]
    fn ruby_frames_are_fixed_points() {
        if std::process::Command::new("ruby")
            .arg("-v")
            .output()
            .is_err()
        {
            eprintln!("ruby not found, skipping");
            return;
        }
        let mut views = vec![
            (110, 48, View::classic(110, 48)),
            (200, 60, View::classic(200, 60)),
            (
                60,
                20,
                View {
                    cx: 5.0,
                    cy: 5.0,
                    sc: 0.01,
                },
            ),
        ];
        let mut v = View::classic(120, 40);
        for _ in 0..12 {
            v.zoom(1.0 / ZOOM_STEP);
        }
        v.pan(-0.37, 0.21);
        views.push((120, 40, v));
        let mut deep = View {
            cx: -0.743643887037151,
            cy: 0.13182590420533,
            sc: 1.0,
        };
        deep.zoom(2e-9);
        views.push((90, 30, deep));
        let dir = std::env::temp_dir();
        for (i, (w, h, view)) in views.into_iter().enumerate() {
            for outside in [false, true] {
                let frame = lang::ruby::Ruby
                    .render(&view.params(w, h, outside))
                    .expect("layout");
                let path = dir.join(format!("mandelview-test-{}-{i}.rb", std::process::id()));
                std::fs::write(&path, &frame.text).unwrap();
                let out = std::process::Command::new("ruby")
                    .arg(&path)
                    .output()
                    .unwrap();
                let _ = std::fs::remove_file(&path);
                assert!(
                    out.status.success(),
                    "frame {i} failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                assert_eq!(
                    String::from_utf8_lossy(&out.stdout),
                    frame.text,
                    "frame {i} is not a fixed point"
                );
            }
        }
    }
}
