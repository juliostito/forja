//! Draws the exercise animations from `forja_core::motion`: a 2D skeleton (keyframes + IK), its bench /
//! wall, dumbbells, a direction arrow and the spine in green while it stays neutral.

use crate::ui::{DONE, EMBER, EMBER2, LINE, STEEL};
use forja_core::motion::{at_phase, motion_for, phase_at, Joints, Motion, Prop, Pt, View};
use macroquad::prelude::*;

const LIMB: Color = Color::new(0.78, 0.81, 0.87, 1.0);
const LIMB_BACK: Color = Color::new(0.46, 0.49, 0.55, 1.0);

/// Draws the animation for `id` with its feet line at (x, y).
pub fn draw(id: &str, x: f32, y: f32, scale: f32, t: f32) {
    let m = motion_for(id);
    let phase = phase_at(&m, t);
    let rising = ((t / m.period).rem_euclid(1.0)) < 0.5;
    draw_motion(&m, &at_phase(&m, phase), rising, x, y, scale);
}

pub fn draw_motion(m: &Motion, j: &Joints, rising: bool, x: f32, y: f32, scale: f32) {
    let o = vec2(x, y);
    let pt = |p: Pt| o + vec2(p.x, p.y) * scale;
    // Props.
    for p in &m.props {
        match *p {
            Prop::Block { x: cx, w, h } => {
                let tl = pt(Pt::new(cx - w / 2.0, -h));
                draw_rectangle(tl.x, tl.y, w * scale, 5.0 * scale, LINE);
                let l1 = pt(Pt::new(cx - w / 2.0 + 3.0, -h));
                let l2 = pt(Pt::new(cx + w / 2.0 - 3.0, -h));
                draw_line(l1.x, l1.y, l1.x, y, 3.0, LINE);
                draw_line(l2.x, l2.y, l2.x, y, 3.0, LINE);
            }
            Prop::Wall { x: wx } => {
                let b = pt(Pt::new(wx, 0.0));
                draw_line(b.x, b.y, b.x, b.y - 110.0 * scale, 4.0, LINE);
            }
        }
    }
    draw_line(x - 75.0 * scale, y, x + 75.0 * scale, y, 2.0, LINE);

    let seg = |a: Pt, b: Pt, c: Color, w: f32| {
        let (u, v) = (pt(a), pt(b));
        draw_line(u.x, u.y, v.x, v.y, w * scale, c);
    };
    let back = if m.view == View::Side {
        LIMB_BACK
    } else {
        LIMB
    };
    // Far limbs first.
    seg(j.hip[1], j.knee[1], back, 4.5);
    seg(j.knee[1], j.ankle[1], back, 4.5);
    seg(j.shoulder[1], j.elbow[1], back, 3.5);
    seg(j.elbow[1], j.wrist[1], back, 3.5);
    if m.view == View::Front {
        seg(j.shoulder[0], j.shoulder[1], LIMB, 4.0);
        seg(j.hip[0], j.hip[1], LIMB, 4.0);
    }
    // Spine: green while neutral, ember for mobility drills that move it on purpose.
    seg(
        j.root,
        j.neck,
        if m.neutral_spine { DONE } else { EMBER },
        6.0,
    );
    seg(j.hip[0], j.knee[0], LIMB, 5.0);
    seg(j.knee[0], j.ankle[0], LIMB, 5.0);
    seg(j.shoulder[0], j.elbow[0], LIMB, 4.0);
    seg(j.elbow[0], j.wrist[0], LIMB, 4.0);
    let h = pt(j.head);
    draw_circle(h.x, h.y, 7.5 * scale, LIMB);
    for i in 0..2 {
        if m.weights[i] {
            let w = pt(j.wrist[i]);
            draw_rectangle(
                w.x - 7.0 * scale,
                w.y - 2.5 * scale,
                14.0 * scale,
                5.0 * scale,
                STEEL,
            );
            draw_rectangle(
                w.x - 8.0 * scale,
                w.y - 4.0 * scale,
                3.0 * scale,
                8.0 * scale,
                STEEL,
            );
            draw_rectangle(
                w.x + 5.0 * scale,
                w.y - 4.0 * scale,
                3.0 * scale,
                8.0 * scale,
                STEEL,
            );
        }
    }

    // Direction arrow on the joint that travels the most.
    let a = at_phase(m, 0.0);
    let b = at_phase(m, 1.0);
    let cands = [
        (a.wrist[0], b.wrist[0], j.wrist[0]),
        (a.root, b.root, j.root),
        (a.ankle[0], b.ankle[0], j.ankle[0]),
        (a.ankle[1], b.ankle[1], j.ankle[1]),
        (a.neck, b.neck, j.neck),
    ];
    if let Some((from, to, now)) = cands
        .iter()
        .copied()
        .max_by(|p, q| p.0.dist(p.1).total_cmp(&q.0.dist(q.1)))
    {
        let d = vec2(to.x - from.x, to.y - from.y);
        if d.length() > 2.0 {
            let dir = d.normalize() * if rising { 1.0 } else { -1.0 };
            let c = pt(now) + vec2(-dir.y, dir.x) * 14.0 * scale;
            let tail = c - dir * 10.0 * scale;
            let tip = c + dir * 10.0 * scale;
            draw_line(tail.x, tail.y, tip.x, tip.y, 2.0, EMBER2);
            let perp = vec2(-dir.y, dir.x);
            draw_triangle(
                tip + dir * 5.0 * scale,
                tip + perp * 4.0 * scale,
                tip - perp * 4.0 * scale,
                EMBER2,
            );
        }
    }
}
