//! CircuitJS1 `interpPoint` / `interpPoint2` geometry.

pub fn dsign(p1: (i32, i32), p2: (i32, i32)) -> i32 {
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    if dy == 0 {
        dx.signum()
    } else {
        dy.signum()
    }
}

#[allow(dead_code)]
pub fn interp(a: (i32, i32), b: (i32, i32), f: f64) -> (i32, i32) {
    (
        (a.0 as f64 * (1.0 - f) + b.0 as f64 * f).round() as i32,
        (a.1 as f64 * (1.0 - f) + b.1 as f64 * f).round() as i32,
    )
}

/// Point at fraction `f` along `a→b`, offset `off` perpendicular (CircuitJS1 units).
pub fn interp_off(a: (i32, i32), b: (i32, i32), f: f64, off: f64) -> (i32, i32) {
    let dx = (b.0 - a.0) as f64;
    let dy = (b.1 - a.1) as f64;
    let dn = (dx * dx + dy * dy).sqrt().max(1e-9);
    let x = a.0 as f64 * (1.0 - f) + b.0 as f64 * f + dy / dn * off;
    let y = a.1 as f64 * (1.0 - f) + b.1 as f64 * f - dx / dn * off;
    (x.round() as i32, y.round() as i32)
}

pub fn interp2(a: (i32, i32), b: (i32, i32), f: f64, hs: f64) -> ((i32, i32), (i32, i32)) {
    (interp_off(a, b, f, hs), interp_off(a, b, f, -hs))
}

/// CircuitJS1 `CustomLogicModel.unescape`.
pub fn unescape(s: &str) -> String {
    if s == "\\0" {
        return String::new();
    }
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            let c = b[i + 1] as char;
            out.push(match c {
                'n' => '\n',
                'r' => '\r',
                's' => ' ',
                'p' => '+',
                'q' => '=',
                'h' => '#',
                'a' => '&',
                other => other,
            });
            i += 2;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// VCCS/VCVS chip posts: A, B on the west side, outputs on the east.
pub fn analog_chip_posts(p1: (i32, i32), flags: i32) -> Vec<(i32, i32)> {
    let csize = if (flags & 1) != 0 { 1 } else { 2 };
    let cspc2 = 16 * csize;
    let x0 = p1.0 + cspc2;
    let y0 = p1.1;
    vec![
        (p1.0, y0),
        (p1.0, y0 + cspc2),
        (x0 + 2 * cspc2, y0),
        (x0 + 2 * cspc2, y0 + cspc2),
    ]
}

pub fn parse_linear_gain(expr: &str) -> f64 {
    let s = unescape(expr);
    let s = s.trim();
    if let Some(idx) = s.find('*') {
        s[..idx].trim().parse().unwrap_or(0.1)
    } else {
        s.parse().unwrap_or(0.1)
    }
}

pub const FLAG_TRANSFORMER_REVERSE: i32 = 4;
pub const FLAG_TRANSFORMER_VERTICAL: i32 = 8;
pub const FLAG_TRANSFORMER_FLIP: i32 = 16;

/// Transformer / gyrator posts: winding 1 is 0–2, winding 2 is 1–3.
pub fn transformer_posts(p1: (i32, i32), p2: (i32, i32), flags: i32) -> Vec<(i32, i32)> {
    let vertical = (flags & FLAG_TRANSFORMER_VERTICAL) != 0;
    let width = if vertical {
        -((p2.0 - p1.0).abs().max(32))
    } else {
        (p2.1 - p1.1).abs().max(32)
    };
    let flip = if (flags & FLAG_TRANSFORMER_FLIP) != 0 {
        -1
    } else {
        1
    };
    let off = -(dsign(p1, p2) * width * flip) as f64;
    let mut posts = vec![
        p1,
        p2,
        interp_off(p1, p2, 0.0, off),
        interp_off(p1, p2, 1.0, off),
    ];
    if (flags & FLAG_TRANSFORMER_REVERSE) != 0 {
        posts.swap(1, 3);
    }
    posts
}

pub fn tapped_transformer_posts(p1: (i32, i32), p2: (i32, i32), flags: i32) -> Vec<(i32, i32)> {
    let flip = if (flags & 1) != 0 { -1.0 } else { 1.0 };
    let hs = 32.0 * flip;
    vec![
        p1,
        interp_off(p1, p2, 0.0, -hs * 2.0),
        p2,
        interp_off(p1, p2, 1.0, -hs),
        interp_off(p1, p2, 1.0, -hs * 2.0),
    ]
}

pub fn analog_switch_posts(p1: (i32, i32), p2: (i32, i32)) -> Vec<(i32, i32)> {
    vec![p1, p2, interp_off(p1, p2, 0.5, -16.0)]
}

pub fn analog_switch2_posts(p1: (i32, i32), p2: (i32, i32)) -> Vec<(i32, i32)> {
    let (t0, t1) = interp2(p1, p2, 1.0, 16.0);
    vec![p1, t0, t1, interp_off(p1, p2, 0.5, 16.0)]
}

/// CCII posts: X and Y on the west side, Z on the east (ChipElm sizeX=2, sizeY=3).
pub fn cc2_posts(p1: (i32, i32), flags: i32) -> Vec<(i32, i32)> {
    let csize = if (flags & 1) != 0 { 1 } else { 2 };
    let cspc = 8 * csize;
    let cspc2 = cspc * 2;
    vec![
        (p1.0, p1.1),
        (p1.0, p1.1 + 2 * cspc),
        (p1.0 + 2 * cspc2, p1.1 + cspc),
    ]
}

pub fn pot_wiper(p1: (i32, i32), p2: (i32, i32), flags: i32) -> (i32, i32) {
    const GRID: i32 = 8;
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let flip = (flags & 2) != 0;
    let mut offset = if (dx.abs() > dy.abs()) != flip {
        if dx < 0 {
            dy
        } else {
            -dy
        }
    } else if dy != 0 {
        if dy > 0 {
            dx
        } else {
            -dx
        }
    } else {
        0
    };
    if offset == 0 {
        offset = if (flags & 4) != 0 { -GRID } else { GRID };
    }
    interp_off(p1, p2, 0.5, offset as f64)
}
