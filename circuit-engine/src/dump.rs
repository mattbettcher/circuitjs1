use crate::circuit::Circuit;
use crate::element::Element;
use crate::elements::{
    Capacitor, CurrentElm, Diode, DiodeModel, DiodeStyle, Ground, Inductor, LabeledNode, MosfetElm,
    OpAmpElm, PotElm, ProbeElm, Resistor, Switch2Elm, SwitchElm, TransistorElm, VccsElm, VcvsElm,
    VoltageElm, Wire, FLAG_FWDROP, FLAG_MODEL,
};
use crate::error::{Result, SimError};
use crate::geom::{parse_linear_gain, unescape};

/// Parse a CircuitJS1 text dump (wave-1/2 subset plus r,c,l,v,i,w,g,d).
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
            'o' | 'h' | '!' | '%' | '?' | 'B' | '&' | 'x' | 'b' => continue,
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

        // Models and graphics.
        if dump_type == 34 || dump_type == 32 || dump_type == 38 || dump_type == 423 {
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
            t if t == b'c' as i32 => Box::new(parse_cap(&mut tok, x1, y1, x2, y2, flags)),
            209 => Box::new(parse_polar(&mut tok, x1, y1, x2, y2, flags)),
            t if t == b'l' as i32 => {
                let l = parse_f(&mut tok, "inductance")?;
                let cur = tok.next().map(|s| s.parse().unwrap_or(0.0)).unwrap_or(0.0);
                let ic = tok.next().map(|s| s.parse().unwrap_or(0.0)).unwrap_or(0.0);
                Box::new(Inductor::with_state(x1, y1, x2, y2, flags, l, cur, ic))
            }
            t if t == b'v' as i32 => Box::new(parse_voltage(&mut tok, x1, y1, x2, y2, flags, false)),
            t if t == b'R' as i32 => Box::new(parse_voltage(&mut tok, x1, y1, x2, y2, flags, true)),
            t if t == b'i' as i32 => {
                let cur = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.01);
                Box::new(CurrentElm::new(x1, y1, x2, y2, cur))
            }
            t if t == b'w' as i32 => Box::new(Wire::with_flags(x1, y1, x2, y2, flags)),
            t if t == b'g' as i32 => Box::new(Ground::with_flags(x1, y1, x2, y2, flags)),
            t if t == b'd' as i32 => {
                let model = parse_diode_model(&mut tok, flags, None);
                Box::new(Diode::with_style(
                    x1,
                    y1,
                    x2,
                    y2,
                    flags,
                    model,
                    DiodeStyle::Junction,
                ))
            }
            t if t == b'z' as i32 => {
                let extra_z = if (flags & FLAG_MODEL) == 0 {
                    tok.next().and_then(|s| s.parse().ok())
                } else {
                    None
                };
                let model = if let Some(zv) = extra_z {
                    DiodeModel::from_fwdrop(0.805904783, zv)
                } else {
                    parse_diode_model(&mut tok, flags, Some(DiodeModel::default_zener()))
                };
                Box::new(Diode::with_style(
                    x1,
                    y1,
                    x2,
                    y2,
                    flags,
                    model,
                    DiodeStyle::Zener,
                ))
            }
            162 => {
                let model = if (flags & (FLAG_MODEL | FLAG_FWDROP)) == 0 {
                    DiodeModel::from_fwdrop(2.1024259, 0.0)
                } else {
                    parse_diode_model(&mut tok, flags, Some(DiodeModel::default_led()))
                };
                // RGB / brightness tokens ignored electrically
                let _ = tok.next();
                let _ = tok.next();
                let _ = tok.next();
                Box::new(Diode::with_style(
                    x1,
                    y1,
                    x2,
                    y2,
                    flags,
                    model,
                    DiodeStyle::Led,
                ))
            }
            t if t == b's' as i32 => {
                let pos = parse_switch_pos(tok.next().unwrap_or("0"));
                let mom = tok.next().is_some_and(|s| s.eq_ignore_ascii_case("true"));
                let label = tok.next().map(|s| unescape(s));
                Box::new(SwitchElm::from_dump(x1, y1, x2, y2, flags, pos, mom, label))
            }
            t if t == b'S' as i32 => {
                let pos = parse_switch_pos(tok.next().unwrap_or("0"));
                let mom = tok.next().is_some_and(|s| s.eq_ignore_ascii_case("true"));
                let link = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let throws = tok.next().and_then(|s| s.parse().ok()).unwrap_or(2);
                Box::new(Switch2Elm::from_dump(
                    x1, y1, x2, y2, flags, pos, mom, link, throws,
                ))
            }
            207 => {
                let rest: Vec<&str> = tok.collect();
                let text = if rest.is_empty() {
                    "label".into()
                } else if (flags & 4) != 0 {
                    unescape(rest[0])
                } else {
                    rest.join(" ")
                };
                Box::new(LabeledNode::from_dump(x1, y1, x2, y2, flags, text))
            }
            174 => {
                let max_r = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1000.0);
                let pos = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
                let rest: Vec<&str> = tok.collect();
                let slider = if rest.is_empty() {
                    "Resistance".into()
                } else {
                    rest.join(" ")
                };
                Box::new(PotElm::from_dump(x1, y1, x2, y2, flags, max_r, pos, slider))
            }
            t if t == b'O' as i32 => Box::new(ProbeElm::output(x1, y1, x2, y2, flags)),
            t if t == b'p' as i32 => Box::new(ProbeElm::probe(x1, y1, x2, y2, flags)),
            368 => Box::new(ProbeElm::test_point(x1, y1, x2, y2, flags)),
            370 => Box::new(ProbeElm::ammeter(x1, y1, x2, y2, flags)),
            t if t == b'a' as i32 => {
                let max_out = tok.next().and_then(|s| s.parse().ok()).unwrap_or(15.0);
                let min_out = tok.next().and_then(|s| s.parse().ok()).unwrap_or(-15.0);
                let gbw = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1e6);
                let _v0 = tok.next();
                let _v1 = tok.next();
                let gain = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1e5);
                Box::new(OpAmpElm::from_dump(
                    x1, y1, x2, y2, flags, max_out, min_out, gbw, gain,
                ))
            }
            212 => {
                let _ic = tok.next();
                let expr = tok.next().unwrap_or("1*(a-b)");
                Box::new(VcvsElm::new(
                    x1,
                    y1,
                    x2,
                    y2,
                    flags,
                    parse_linear_gain(expr),
                ))
            }
            213 => {
                let _ic = tok.next();
                let expr = tok.next().unwrap_or(".1*(a-b)");
                Box::new(VccsElm::new(
                    x1,
                    y1,
                    x2,
                    y2,
                    flags,
                    parse_linear_gain(expr),
                ))
            }
            t if t == b't' as i32 => {
                let pnp = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                let last_vbe = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let last_vbc = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let beta = tok.next().and_then(|s| s.parse().ok()).unwrap_or(100.0);
                Box::new(TransistorElm::from_dump(
                    x1, y1, x2, y2, flags, pnp, last_vbe, last_vbc, beta,
                ))
            }
            t if t == b'f' as i32 => {
                let vt = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1.5);
                let beta = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);
                Box::new(MosfetElm::from_dump(x1, y1, x2, y2, flags, vt, beta))
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

fn parse_cap<'a>(
    tok: &mut impl Iterator<Item = &'a str>,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    flags: i32,
) -> Capacitor {
    let cap = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1e-5);
    let voltdiff = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let iv = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1e-3);
    let sr = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Capacitor::with_state(x1, y1, x2, y2, flags, cap, voltdiff, iv, sr)
}

fn parse_polar<'a>(
    tok: &mut impl Iterator<Item = &'a str>,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    flags: i32,
) -> Capacitor {
    let cap = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1e-5);
    let voltdiff = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let iv = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1e-3);
    let sr = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let mn = tok.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    Capacitor::polar(x1, y1, x2, y2, flags, cap, voltdiff, iv, sr, mn)
}

fn parse_voltage<'a>(
    tok: &mut impl Iterator<Item = &'a str>,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    flags: i32,
    rail: bool,
) -> VoltageElm {
    let wf = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let freq = tok.next().and_then(|s| s.parse().ok()).unwrap_or(40.0);
    let maxv = tok.next().and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let bias = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let phase = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let duty = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
    if rail {
        VoltageElm::from_dump_rail(x1, y1, x2, y2, flags, wf, freq, maxv, bias, phase, duty)
    } else {
        VoltageElm::from_dump(x1, y1, x2, y2, flags, wf, freq, maxv, bias, phase, duty)
    }
}

fn parse_diode_model<'a>(
    tok: &mut impl Iterator<Item = &'a str>,
    flags: i32,
    fallback: Option<DiodeModel>,
) -> DiodeModel {
    if (flags & FLAG_MODEL) != 0 {
        let name = tok.next().unwrap_or("default");
        match name {
            "default-zener" => DiodeModel::default_zener(),
            "default-led" => DiodeModel::default_led(),
            _ => fallback.unwrap_or_else(DiodeModel::default_model),
        }
    } else if (flags & FLAG_FWDROP) != 0 {
        let fw = tok
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.805904783);
        DiodeModel::from_fwdrop(fw, 0.0)
    } else {
        fallback.unwrap_or_else(DiodeModel::default_model)
    }
}

fn parse_switch_pos(s: &str) -> i32 {
    match s {
        "true" => 1,
        "false" => 0,
        other => other.parse().unwrap_or(0),
    }
}

fn parse_options<'a>(circuit: &mut Circuit, mut tok: impl Iterator<Item = &'a str>) -> Result<()> {
    let flags: i32 = parse_i(&mut tok, "option-flags")?;
    if let Some(dt) = tok.next().and_then(|s| s.parse().ok()) {
        circuit.set_time_step(dt);
    }
    let _ = tok.next();
    let _ = tok.next();
    let _ = tok.next();
    let _ = tok.next();
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
