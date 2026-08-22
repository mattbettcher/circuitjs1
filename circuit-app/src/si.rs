/// Format `x` with an SI prefix and unit, e.g. `1 kΩ`, `4.473 mA`.
pub fn si(x: f64, unit: &str) -> String {
    if !x.is_finite() {
        return format!("? {unit}");
    }
    let ax = x.abs();
    let (scale, prefix) = if ax == 0.0 {
        (1.0, "")
    } else if ax >= 1e9 {
        (1e-9, "G")
    } else if ax >= 1e6 {
        (1e-6, "M")
    } else if ax >= 1e3 {
        (1e-3, "k")
    } else if ax >= 1.0 {
        (1.0, "")
    } else if ax >= 1e-3 {
        (1e3, "m")
    } else if ax >= 1e-6 {
        (1e6, "µ")
    } else if ax >= 1e-9 {
        (1e9, "n")
    } else if ax >= 1e-12 {
        (1e12, "p")
    } else {
        (1e15, "f")
    };
    let v = x * scale;
    let body = if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.3}")
    };
    let body = body.trim_end_matches('0').trim_end_matches('.');
    if prefix.is_empty() {
        format!("{body} {unit}")
    } else {
        format!("{body} {prefix}{unit}")
    }
}

pub fn time(t: f64) -> String {
    si(t, "s")
}
