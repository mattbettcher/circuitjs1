use crate::circuit::Circuit;
use crate::element::Element;
use crate::elements::{
    Capacitor, CurrentElm, Diode, DiodeModel, Ground, Inductor, Resistor, VoltageElm, Wire,
    FLAG_FWDROP, FLAG_MODEL,
};
use crate::error::{Result, SimError};

/// Parse a CircuitJS1 text dump (subset: r, c, l, v, i, w, g, d, and `$` options).
pub fn parse_dump(text: &str) -> Result<Circuit> {
    let mut circuit = Circuit::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let mut tok = line.split_whitespace();
        let Some(kind) = tok.next() else {
            continue;
        };
        let first = kind.chars().next().unwrap_or('\0');
        match first {
            'o' | 'h' | '!' | '%' | '?' | 'B' | '&' => continue,
            '$' => {
                parse_options(&mut circuit, tok)?;
                continue;
            }
            _ => {}
        }

        let dump_type = if first.is_ascii_digit() {
            kind.parse::<i32>()
                .map_err(|_| SimError::Parse(format!("bad dump type {kind}")))?
        } else if kind.len() == 1 {
            first as i32
        } else {
            return Err(SimError::Parse(format!("unrecognized dump type {kind}")));
        };

        // Model dumps (34 = diode model, 32 = transistor, ...).
        if dump_type == 34 || dump_type == 32 || dump_type == 38 {
            continue;
        }

        let x1: i32 = parse_i(&mut tok, "x1")?;
        let y1: i32 = parse_i(&mut tok, "y1")?;
        let x2: i32 = parse_i(&mut tok, "x2")?;
        let y2: i32 = parse_i(&mut tok, "y2")?;
        let flags: i32 = parse_i(&mut tok, "flags")?;

        let elm: Box<dyn Element> = match dump_type {
            t if t == b'r' as i32 => {
                let r = parse_f(&mut tok, "resistance")?;
                Box::new(Resistor::new(x1, y1, x2, y2, r))
            }
            t if t == b'c' as i32 => {
                let cap = parse_f(&mut tok, "capacitance")?;
                let voltdiff = tok.next().map(|s| s.parse().unwrap_or(0.0)).unwrap_or(0.0);
                let iv = tok
                    .next()
                    .map(|s| s.parse().unwrap_or(1e-3))
                    .unwrap_or(1e-3);
                let sr = tok.next().map(|s| s.parse().unwrap_or(0.0)).unwrap_or(0.0);
                Box::new(Capacitor::with_state(
                    x1, y1, x2, y2, flags, cap, voltdiff, iv, sr,
                ))
            }
            t if t == b'l' as i32 => {
                let l = parse_f(&mut tok, "inductance")?;
                let cur = tok.next().map(|s| s.parse().unwrap_or(0.0)).unwrap_or(0.0);
                let ic = tok.next().map(|s| s.parse().unwrap_or(0.0)).unwrap_or(0.0);
                Box::new(Inductor::with_state(x1, y1, x2, y2, flags, l, cur, ic))
            }
            t if t == b'v' as i32 => {
                let wf = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let freq = tok.next().and_then(|s| s.parse().ok()).unwrap_or(40.0);
                let maxv = tok.next().and_then(|s| s.parse().ok()).unwrap_or(5.0);
                let bias = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let phase = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let duty = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
                Box::new(VoltageElm::from_dump(
                    x1, y1, x2, y2, flags, wf, freq, maxv, bias, phase, duty,
                ))
            }
            t if t == b'i' as i32 => {
                let cur = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.01);
                Box::new(CurrentElm::new(x1, y1, x2, y2, cur))
            }
            t if t == b'w' as i32 => Box::new(Wire::with_flags(x1, y1, x2, y2, flags)),
            t if t == b'g' as i32 => Box::new(Ground::with_flags(x1, y1, x2, y2, flags)),
            t if t == b'd' as i32 => {
                let model = if (flags & FLAG_MODEL) != 0 {
                    let name = tok.next().unwrap_or("default");
                    if name == "default" {
                        DiodeModel::default_model()
                    } else {
                        DiodeModel::default_model()
                    }
                } else if (flags & FLAG_FWDROP) != 0 {
                    let fw = tok
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.805904783);
                    DiodeModel::from_fwdrop(fw, 0.0)
                } else {
                    DiodeModel::default_model()
                };
                Box::new(Diode::with_model(x1, y1, x2, y2, flags, model))
            }
            other => {
                return Err(SimError::Parse(format!(
                    "unsupported element dump type {other}"
                )));
            }
        };
        circuit.push(elm);
    }
    Ok(circuit)
}

fn parse_options<'a>(circuit: &mut Circuit, mut tok: impl Iterator<Item = &'a str>) -> Result<()> {
    let flags: i32 = parse_i(&mut tok, "option-flags")?;
    if let Some(dt) = tok.next().and_then(|s| s.parse().ok()) {
        circuit.set_time_step(dt);
    }
    // skip iterationSpeed, currentBar, voltageRange
    let _ = tok.next();
    let _ = tok.next();
    let _ = tok.next();
    let _ = tok.next(); // powerBar
    if let Some(min_dt) = tok.next().and_then(|s| s.parse().ok()) {
        circuit.min_time_step = min_dt;
    }
    circuit.set_adjust_time_step((flags & 64) != 0);
    Ok(())
}

fn parse_i<'a>(tok: &mut impl Iterator<Item = &'a str>, what: &str) -> Result<i32> {
    tok.next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| SimError::Parse(format!("missing {what}")))
}

fn parse_f<'a>(tok: &mut impl Iterator<Item = &'a str>, what: &str) -> Result<f64> {
    tok.next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| SimError::Parse(format!("missing {what}")))
}
