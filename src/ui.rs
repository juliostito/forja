//! Palette, fonts and small immediate-mode widgets for the demo.

use macroquad::prelude::*;

pub const W: f32 = 1280.0;
pub const H: f32 = 840.0;

// "La Forja" palette (section 4.1).
pub const IRON: Color = Color::new(0.059, 0.071, 0.090, 1.0); // #0f1217
pub const PANEL: Color = Color::new(0.090, 0.106, 0.133, 1.0); // #171b22
pub const PANEL2: Color = Color::new(0.125, 0.145, 0.180, 1.0);
pub const HOVER: Color = Color::new(0.20, 0.23, 0.28, 1.0);
pub const LINE: Color = Color::new(0.25, 0.28, 0.33, 1.0);
pub const STEEL: Color = Color::new(0.596, 0.635, 0.702, 1.0); // #98a2b3
pub const EMBER: Color = Color::new(1.0, 0.478, 0.102, 1.0); // #ff7a1a
pub const EMBER2: Color = Color::new(1.0, 0.812, 0.302, 1.0); // #ffcf4d
pub const TEMPER: Color = Color::new(0.490, 0.612, 1.0, 1.0); // #7d9cff
pub const DONE: Color = Color::new(0.373, 0.816, 0.561, 1.0); // #5fd08f
pub const TEXT: Color = Color::new(0.902, 0.914, 0.937, 1.0);
pub const MUTED: Color = Color::new(0.45, 0.48, 0.54, 1.0);
pub const ALERT: Color = Color::new(1.0, 0.33, 0.30, 1.0);

#[derive(Clone, Copy, PartialEq)]
pub enum Face {
    /// Condensed display (Bahnschrift stands in for Barlow Condensed on Windows).
    Display,
    Body,
    Bold,
    /// Tabular figures.
    Mono,
}

pub struct Fonts {
    display: Option<Font>,
    body: Option<Font>,
    bold: Option<Font>,
    mono: Option<Font>,
}

impl Fonts {
    /// System fonts read at run time (nothing bundled); falls back to the built-in font.
    pub fn load() -> Self {
        let load = |name: &str| {
            let path = format!("C:\\Windows\\Fonts\\{name}");
            std::fs::read(path)
                .ok()
                .and_then(|b| load_ttf_font_from_bytes(&b).ok())
        };
        Fonts {
            display: load("bahnschrift.ttf"),
            body: load("segoeui.ttf"),
            bold: load("segoeuib.ttf"),
            mono: load("consola.ttf"),
        }
    }

    fn get(&self, f: Face) -> Option<&Font> {
        match f {
            Face::Display => self.display.as_ref().or(self.bold.as_ref()),
            Face::Body => self.body.as_ref(),
            Face::Bold => self.bold.as_ref().or(self.body.as_ref()),
            Face::Mono => self.mono.as_ref().or(self.body.as_ref()),
        }
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, color: Color, face: Face) {
        let params = TextParams {
            font: self.get(face),
            font_size: size as u16,
            color,
            ..Default::default()
        };
        draw_text_ex(s, x, y, params);
    }

    pub fn width(&self, s: &str, size: f32, face: Face) -> f32 {
        measure_text(s, self.get(face), size as u16, 1.0).width
    }

    /// Word-wrapped text; returns the height used.
    #[allow(clippy::too_many_arguments)]
    pub fn wrap(
        &self,
        s: &str,
        x: f32,
        y: f32,
        max_w: f32,
        size: f32,
        color: Color,
        face: Face,
    ) -> f32 {
        let mut line = String::new();
        let mut yy = y;
        for word in s.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if self.width(&candidate, size, face) > max_w && !line.is_empty() {
                self.text(&line, x, yy, size, color, face);
                yy += size * 1.3;
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            self.text(&line, x, yy, size, color, face);
            yy += size * 1.3;
        }
        yy - y
    }
}

/// Draws a button; returns true when it was clicked this frame.
pub fn draw_button(f: &Fonts, r: Rect, label: &str, primary: bool, enabled: bool) -> bool {
    let hover = enabled && r.contains(mouse_position().into());
    let bg = match (primary, enabled, hover) {
        (_, false, _) => PANEL2,
        (true, _, true) => EMBER2,
        (true, _, false) => EMBER,
        (false, _, true) => HOVER,
        (false, _, false) => PANEL2,
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    if !primary {
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, LINE);
    }
    let size = 17.0;
    let tw = f.width(label, size, Face::Bold);
    let fg = if primary && enabled {
        IRON
    } else if enabled {
        TEXT
    } else {
        MUTED
    };
    f.text(
        label,
        r.x + (r.w - tw) / 2.0,
        r.y + r.h / 2.0 + 6.0,
        size,
        fg,
        Face::Bold,
    );
    hover && is_mouse_button_released(MouseButton::Left)
}

/// Text cut with an ellipsis to fit `max_w`.
#[allow(clippy::too_many_arguments)]
pub fn fit_text(
    f: &Fonts,
    s: &str,
    x: f32,
    y: f32,
    size: f32,
    max_w: f32,
    color: Color,
    face: Face,
) {
    if f.width(s, size, face) <= max_w {
        f.text(s, x, y, size, color, face);
        return;
    }
    let mut cut: String = s.to_string();
    while !cut.is_empty() && f.width(&format!("{cut}…"), size, face) > max_w {
        cut.pop();
    }
    f.text(&format!("{cut}…"), x, y, size, color, face);
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Check mark (shape + colour, never colour alone).
pub fn draw_check(c: Vec2, s: f32, color: Color) {
    draw_line(c.x - s, c.y, c.x - s * 0.3, c.y + s * 0.7, 2.5, color);
    draw_line(
        c.x - s * 0.3,
        c.y + s * 0.7,
        c.x + s,
        c.y - s * 0.7,
        2.5,
        color,
    );
}

/// Disclosure chevron: right when closed, down when open.
pub fn draw_chevron(c: Vec2, open: bool, color: Color) {
    if open {
        draw_triangle(
            vec2(c.x - 5.0, c.y - 3.0),
            vec2(c.x + 5.0, c.y - 3.0),
            vec2(c.x, c.y + 4.0),
            color,
        );
    } else {
        draw_triangle(
            vec2(c.x - 3.0, c.y - 5.0),
            vec2(c.x - 3.0, c.y + 5.0),
            vec2(c.x + 4.0, c.y),
            color,
        );
    }
}

/// Countdown ring for the rest timer.
pub fn draw_rest_ring(f: &Fonts, c: Vec2, left: f32, total: f32) {
    draw_circle_lines(c.x, c.y, 30.0, 4.0, PANEL2);
    let frac = (left / total).clamp(0.0, 1.0);
    let segs = 48;
    let tau = std::f32::consts::TAU;
    for i in 0..(segs as f32 * frac).ceil() as i32 {
        let a0 = -std::f32::consts::FRAC_PI_2 + i as f32 / segs as f32 * tau;
        let a1 = a0 + tau / segs as f32;
        draw_line(
            c.x + a0.cos() * 30.0,
            c.y + a0.sin() * 30.0,
            c.x + a1.cos() * 30.0,
            c.y + a1.sin() * 30.0,
            4.0,
            TEMPER,
        );
    }
    let s = format!("{}", left.ceil() as i32);
    f.text(
        &s,
        c.x - f.width(&s, 24.0, Face::Mono) / 2.0,
        c.y + 8.0,
        24.0,
        TEXT,
        Face::Mono,
    );
    f.text("descanso", c.x - 112.0, c.y + 6.0, 15.0, STEEL, Face::Body);
}
