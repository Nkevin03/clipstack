//! Generates the original Clipstack app icon (1024×1024 PNG) procedurally.
//!
//!     cargo run --example make_icon -- icons/app-icon-1024.png
//!
//! Design: macOS-style rounded square with an indigo→violet gradient and three fanned
//! "clip cards"; the front card has a coloured header strip like the cards in the panel.

use image::{Rgba, RgbaImage};

const S: f32 = 1024.0;

#[derive(Clone, Copy)]
struct C(f32, f32, f32, f32); // straight RGBA 0..1

fn hex(h: u32, a: f32) -> C {
    C(((h >> 16) & 255) as f32 / 255.0, ((h >> 8) & 255) as f32 / 255.0, (h & 255) as f32 / 255.0, a)
}

fn mix(a: C, b: C, t: f32) -> C {
    C(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t, a.3 + (b.3 - a.3) * t)
}

/// Source-over compositing.
fn over(dst: C, src: C) -> C {
    let a = src.3 + dst.3 * (1.0 - src.3);
    if a <= 0.0 {
        return C(0.0, 0.0, 0.0, 0.0);
    }
    let f = |s: f32, d: f32| (s * src.3 + d * dst.3 * (1.0 - src.3)) / a;
    C(f(src.0, dst.0), f(src.1, dst.1), f(src.2, dst.2), a)
}

/// Signed distance to a rounded rectangle centred at (cx, cy), rotated by `rot` radians.
fn sd_rrect(px: f32, py: f32, cx: f32, cy: f32, w: f32, h: f32, r: f32, rot: f32) -> f32 {
    let (s, c) = rot.sin_cos();
    let dx = px - cx;
    let dy = py - cy;
    let lx = dx * c + dy * s;
    let ly = -dx * s + dy * c;
    let qx = lx.abs() - (w / 2.0 - r);
    let qy = ly.abs() - (h / 2.0 - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Local (un-rotated) coordinates relative to a card's centre.
fn local(px: f32, py: f32, cx: f32, cy: f32, rot: f32) -> (f32, f32) {
    let (s, c) = rot.sin_cos();
    let dx = px - cx;
    let dy = py - cy;
    (dx * c + dy * s, -dx * s + dy * c)
}

fn cov(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

struct Card {
    cx: f32,
    cy: f32,
    rot: f32,
    fill: C,
    header: Option<C>,
}

fn pixel(px: f32, py: f32) -> C {
    let mut out = C(0.0, 0.0, 0.0, 0.0);

    // --- Body: macOS icon grid (824 pt body, 100 pt margin), continuous-ish corners.
    let body = sd_rrect(px, py, 512.0, 512.0, 824.0, 824.0, 186.0, 0.0);
    // Drop shadow of the body.
    let sh = sd_rrect(px, py, 512.0, 524.0, 824.0, 824.0, 186.0, 0.0);
    out = over(out, C(0.0, 0.0, 0.0, 0.28 * (1.0 - smoothstep(-6.0, 34.0, sh))));
    let a = cov(body);
    if a > 0.0 {
        // Diagonal gradient indigo → violet, with a soft top highlight.
        let t = ((px - 100.0) + (py - 100.0)) / (2.0 * 824.0);
        let mut g = mix(hex(0x4F46E5, 1.0), hex(0x8B3DF0, 1.0), t.clamp(0.0, 1.0));
        let hl = (1.0 - ((py - 100.0) / 420.0)).clamp(0.0, 1.0) * 0.16;
        g = mix(g, hex(0xFFFFFF, 1.0), hl);
        // Subtle vignette at the bottom.
        let vg = smoothstep(560.0, 924.0, py) * 0.12;
        g = mix(g, hex(0x1E1B4B, 1.0), vg);
        out = over(out, C(g.0, g.1, g.2, a));
        // Inner hairline highlight.
        let rim = (1.0 - ((body + 3.0).abs() / 1.5)).clamp(0.0, 1.0) * 0.25 * smoothstep(700.0, 120.0, py);
        out = over(out, C(1.0, 1.0, 1.0, rim * a));
    }

    // --- Three fanned cards.
    let w = 430.0;
    let h = 500.0;
    let r = 64.0;
    let cards = [
        Card { cx: 560.0, cy: 470.0, rot: 0.26, fill: hex(0xFFFFFF, 0.38), header: None },
        Card { cx: 530.0, cy: 500.0, rot: 0.12, fill: hex(0xFFFFFF, 0.62), header: None },
        Card { cx: 480.0, cy: 545.0, rot: -0.05, fill: hex(0xFFFFFF, 1.0), header: Some(hex(0xFFB020, 1.0)) },
    ];
    for card in &cards {
        // Card shadow.
        let d_sh = sd_rrect(px, py, card.cx + 6.0, card.cy + 22.0, w, h, r, card.rot);
        out = over(out, C(0.07, 0.04, 0.25, 0.30 * (1.0 - smoothstep(-10.0, 46.0, d_sh)) * card.fill.3.max(0.5)));
        let d = sd_rrect(px, py, card.cx, card.cy, w, h, r, card.rot);
        let ca = cov(d);
        if ca <= 0.0 {
            continue;
        }
        let (lx, ly) = local(px, py, card.cx, card.cy, card.rot);
        let mut col = card.fill;
        if let Some(hc) = card.header {
            // Header strip (top 22% of the card) with its own gradient.
            let top = -h / 2.0;
            if ly < top + 112.0 {
                let t = ((lx + w / 2.0) / w).clamp(0.0, 1.0);
                col = mix(hc, hex(0xFF7A45, 1.0), t);
            } else {
                // Text lines.
                let lines = [(top + 168.0, 0.70), (top + 228.0, 0.86), (top + 288.0, 0.58), (top + 348.0, 0.74)];
                for (i, (y, frac)) in lines.iter().enumerate() {
                    let lw = (w - 120.0) * frac;
                    let x0 = -w / 2.0 + 60.0;
                    let dl = sd_rrect(lx, ly, x0 + lw / 2.0, *y, lw, 26.0, 13.0, 0.0);
                    let la = cov(dl);
                    if la > 0.0 {
                        let shade = if i == 0 { hex(0x312E81, 0.85) } else { hex(0x6B6F86, 0.45) };
                        col = over(col, C(shade.0, shade.1, shade.2, shade.3 * la));
                    }
                }
                // "⌘1"-style badge in the bottom-right corner of the card.
                let db = sd_rrect(lx, ly, w / 2.0 - 92.0, h / 2.0 - 64.0, 92.0, 52.0, 16.0, 0.0);
                let ba = cov(db);
                if ba > 0.0 {
                    col = over(col, C(0.31, 0.27, 0.90, 0.16 * ba));
                }
            }
            // App "icon" dot in the header.
            let di = sd_rrect(lx, ly, -w / 2.0 + 74.0, top + 56.0, 48.0, 48.0, 14.0, 0.0);
            let ia = cov(di);
            if ia > 0.0 {
                col = over(col, C(1.0, 1.0, 1.0, 0.92 * ia));
            }
            let dt = sd_rrect(lx, ly, -w / 2.0 + 190.0, top + 56.0, 150.0, 22.0, 11.0, 0.0);
            let ta = cov(dt);
            if ta > 0.0 {
                col = over(col, C(1.0, 1.0, 1.0, 0.75 * ta));
            }
        }
        out = over(out, C(col.0, col.1, col.2, col.3 * ca));
    }
    out
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "icons/app-icon-1024.png".into());
    let n = S as u32;
    // 2× supersampling for clean edges.
    let img = RgbaImage::from_fn(n, n, |x, y| {
        let mut acc = C(0.0, 0.0, 0.0, 0.0);
        let mut alpha = 0.0;
        for (ox, oy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
            let c = pixel(x as f32 + ox, y as f32 + oy);
            // Premultiplied accumulation.
            acc = C(acc.0 + c.0 * c.3, acc.1 + c.1 * c.3, acc.2 + c.2 * c.3, 0.0);
            alpha += c.3;
        }
        if alpha <= 0.0 {
            return Rgba([0, 0, 0, 0]);
        }
        let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Rgba([to(acc.0 / alpha), to(acc.1 / alpha), to(acc.2 / alpha), to(alpha / 4.0)])
    });
    img.save(&path).expect("write icon");
    println!("wrote {path}");
}
