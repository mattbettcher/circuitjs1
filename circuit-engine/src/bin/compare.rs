//! Run the 15 engine tests and emit JSON traces for plotting vs CircuitJS1.
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use circuit_engine::elements::{
    Capacitor, CurrentElm, Diode, Ground, Inductor, Resistor, VoltageElm, Wire,
};
use circuit_engine::{lu_factor_dense, lu_solve_dense, parse_dump, Circuit, FLAG_BACK_EULER};

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_f64(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.12}")
    } else {
        "null".into()
    }
}

fn json_vec(v: &[f64]) -> String {
    format!(
        "[{}]",
        v.iter().map(|x| json_f64(*x)).collect::<Vec<_>>().join(",")
    )
}

struct Signal {
    name: String,
    unit: String,
    rust: Vec<f64>,
    java: Option<Vec<f64>>,
}

struct Case {
    id: String,
    title: String,
    kind: String,
    notes: String,
    t: Vec<f64>,
    signals: Vec<Signal>,
}

impl Case {
    fn to_json(&self) -> String {
        let sigs: Vec<String> = self
            .signals
            .iter()
            .map(|s| {
                let java = match &s.java {
                    Some(j) => json_vec(j),
                    None => "null".into(),
                };
                format!(
                    "{{\"name\":\"{}\",\"unit\":\"{}\",\"rust\":{},\"java\":{}}}",
                    json_escape(&s.name),
                    json_escape(&s.unit),
                    json_vec(&s.rust),
                    java
                )
            })
            .collect();
        format!(
            "{{\"id\":\"{}\",\"title\":\"{}\",\"kind\":\"{}\",\"notes\":\"{}\",\"t\":{},\"signals\":[{}]}}",
            json_escape(&self.id),
            json_escape(&self.title),
            json_escape(&self.kind),
            json_escape(&self.notes),
            json_vec(&self.t),
            sigs.join(",")
        )
    }
}

fn mat(rows: &[&[f64]]) -> Vec<Vec<f64>> {
    rows.iter().map(|r| r.to_vec()).collect()
}

fn lu_case(id: &str, title: &str, mut a: Vec<Vec<f64>>, mut b: Vec<f64>, labels: &[&str]) -> Case {
    let n = a.len();
    let mut ipvt = vec![0i32; n];
    let ok = lu_factor_dense(&mut a, &mut ipvt);
    if ok {
        lu_solve_dense(&a, &ipvt, &mut b);
    }
    let signals = labels
        .iter()
        .enumerate()
        .map(|(i, name)| Signal {
            name: name.to_string(),
            unit: if name.starts_with('I') { "A" } else { "V" }.into(),
            rust: vec![if ok { b[i] } else { f64::NAN }],
            java: None,
        })
        .collect();
    Case {
        id: id.into(),
        title: title.into(),
        kind: "lu".into(),
        notes: if ok {
            "Crout LU (same routine as SimulationManager.lu_factor_dense)".into()
        } else {
            "singular (factor returned false)".into()
        },
        t: vec![0.0],
        signals,
    }
}

fn divider() -> Circuit {
    let mut c = Circuit::new();
    c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 10.0)));
    c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
    c.push(Box::new(Resistor::new(100, 0, 100, 100, 1000.0)));
    c.push(Box::new(Wire::new(100, 100, 0, 100)));
    c
}

fn sample_every(
    c: &mut Circuit,
    steps: usize,
    stride: usize,
) -> (Vec<f64>, Vec<circuit_engine::Snapshot>) {
    let mut t = Vec::new();
    let mut snaps = Vec::new();
    for i in 0..steps {
        c.step().unwrap();
        if i % stride == 0 || i + 1 == steps {
            let s = c.snapshot();
            t.push(s.t);
            snaps.push(s);
        }
    }
    (t, snaps)
}

fn main() {
    let mut cases = Vec::new();

    cases.push(lu_case(
        "lu_solves_2x2",
        "LU: 2x2 system",
        mat(&[&[2.0, 1.0], &[1.0, 2.0]]),
        vec![3.0, 3.0],
        &["x0", "x1"],
    ));
    cases.push(lu_case(
        "lu_solves_identity",
        "LU: identity",
        mat(&[&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0]]),
        vec![4.0, 5.0, 6.0],
        &["x0", "x1", "x2"],
    ));
    {
        let g = 1.0 / 1000.0;
        cases.push(lu_case(
            "lu_voltage_divider_mna",
            "LU: voltage-divider MNA",
            mat(&[&[g, -g, -1.0], &[-g, 2.0 * g, 0.0], &[1.0, 0.0, 0.0]]),
            vec![0.0, 0.0, 10.0],
            &["Vsource", "Vmid", "Ivs"],
        ));
    }
    {
        let mut a = mat(&[&[1.0, 2.0], &[2.0, 4.0]]);
        let mut ipvt = [0i32; 2];
        let ok = lu_factor_dense(&mut a, &mut ipvt);
        cases.push(Case {
            id: "lu_rejects_singular".into(),
            title: "LU: singular matrix".into(),
            kind: "lu".into(),
            notes: "expect factor=false".into(),
            t: vec![0.0],
            signals: vec![Signal {
                name: "factor_ok".into(),
                unit: "bool".into(),
                rust: vec![if ok { 1.0 } else { 0.0 }],
                java: None,
            }],
        });
    }
    {
        let mut a = mat(&[&[0.0, 0.0], &[1.0, 1.0]]);
        let mut ipvt = [0i32; 2];
        let ok = lu_factor_dense(&mut a, &mut ipvt);
        cases.push(Case {
            id: "lu_rejects_zero_row".into(),
            title: "LU: zero row".into(),
            kind: "lu".into(),
            notes: "expect factor=false".into(),
            t: vec![0.0],
            signals: vec![Signal {
                name: "factor_ok".into(),
                unit: "bool".into(),
                rust: vec![if ok { 1.0 } else { 0.0 }],
                java: None,
            }],
        });
    }

    {
        let mut c = divider();
        c.step().unwrap();
        let s = c.snapshot();
        cases.push(Case {
            id: "voltage_divider_dc".into(),
            title: "Voltage divider DC".into(),
            kind: "dc".into(),
            notes: "10 V, 1 kΩ + 1 kΩ".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "Vmid".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[1][1]],
                    java: None,
                },
                Signal {
                    name: "Vsource".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[1][0]],
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: vec![s.elm_currents[1]],
                    java: None,
                },
            ],
        });
    }

    {
        let mut c = Circuit::new();
        c.push(Box::new(CurrentElm::new(0, 100, 0, 0, 0.002)));
        c.push(Box::new(Resistor::new(0, 0, 0, 100, 2500.0)));
        c.push(Box::new(Ground::new(0, 100, 0, 116)));
        c.step().unwrap();
        let s = c.snapshot();
        cases.push(Case {
            id: "current_source_and_load".into(),
            title: "Current source + load".into(),
            kind: "dc".into(),
            notes: "2 mA into 2.5 kΩ".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "Vload".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[1][0]],
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: vec![s.elm_currents[1]],
                    java: None,
                },
            ],
        });
    }

    {
        let mut c = divider();
        c.analyze().unwrap();
        cases.push(Case {
            id: "wire_merges_nodes".into(),
            title: "Wire merge node count".into(),
            kind: "topo".into(),
            notes: "ground + source+ + mid".into(),
            t: vec![0.0],
            signals: vec![Signal {
                name: "node_count".into(),
                unit: "count".into(),
                rust: vec![c.node_count() as f64],
                java: None,
            }],
        });
    }

    {
        let mut c = Circuit::new();
        c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
        c.step().unwrap();
        let s = c.snapshot();
        cases.push(Case {
            id: "unconnected_resistor".into(),
            title: "Unconnected resistor".into(),
            kind: "dc".into(),
            notes: "1e8 Ω shunt to ground".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "V0".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[0][0]],
                    java: None,
                },
                Signal {
                    name: "V1".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[0][1]],
                    java: None,
                },
            ],
        });
    }

    {
        let mut c = Circuit::new();
        c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 10.0)));
        c.push(Box::new(Resistor::new(0, 0, 0, 100, 1000.0)));
        c.push(Box::new(VoltageElm::dc(200, 100, 200, 0, 3.0)));
        c.push(Box::new(Resistor::new(200, 0, 200, 100, 1000.0)));
        c.push(Box::new(Ground::new(0, 100, 0, 116)));
        c.push(Box::new(Ground::new(200, 100, 200, 116)));
        c.analyze().unwrap();
        let islands = c.island_count() as f64;
        c.step().unwrap();
        let s = c.snapshot();
        cases.push(Case {
            id: "two_grounded_islands".into(),
            title: "Two grounded islands".into(),
            kind: "dc".into(),
            notes: "separate MNA matrices".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "islands".into(),
                    unit: "count".into(),
                    rust: vec![islands],
                    java: None,
                },
                Signal {
                    name: "Vleft".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[1][0]],
                    java: None,
                },
                Signal {
                    name: "Vright".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[3][0]],
                    java: None,
                },
            ],
        });
    }

    {
        let mut c = Circuit::new();
        c.set_time_step(1e-6);
        c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 1.0)));
        c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
        let mut cap = Capacitor::new(100, 0, 100, 100, 1e-6);
        cap.voltdiff = 0.0;
        cap.initial_voltage = 0.0;
        c.push(Box::new(cap));
        c.push(Box::new(Wire::new(100, 100, 0, 100)));
        let (t, snaps) = sample_every(&mut c, 1000, 20);
        cases.push(Case {
            id: "rc_step_response".into(),
            title: "RC step (R=1 kΩ, C=1 µF, dt=1 µs)".into(),
            kind: "transient".into(),
            notes: "trapezoidal companion; 1 ms run".into(),
            t,
            signals: vec![
                Signal {
                    name: "Vc".into(),
                    unit: "V".into(),
                    rust: snaps
                        .iter()
                        .map(|s| s.elm_volts[2][0] - s.elm_volts[2][1])
                        .collect(),
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: snaps.iter().map(|s| s.elm_currents[1]).collect(),
                    java: None,
                },
            ],
        });
    }

    {
        fn lc_circuit(flags: i32) -> Circuit {
            let mut c = Circuit::new();
            c.set_time_step(1e-7);
            let mut cap = Capacitor::new(0, 0, 100, 0, 1e-6);
            cap.voltdiff = 1.0;
            cap.ports.flags = flags;
            c.push(Box::new(Ground::new(0, 0, 0, 16)));
            c.push(Box::new(cap));
            let mut ind = Inductor::new(100, 0, 0, 0, 1e-3);
            ind.ports.flags = flags;
            c.push(Box::new(ind));
            c
        }
        let mut trap = lc_circuit(0);
        let (t, snaps) = sample_every(&mut trap, 2000, 40);
        let mut be = lc_circuit(FLAG_BACK_EULER);
        let (_, be_snaps) = sample_every(&mut be, 2000, 40);
        cases.push(Case {
            id: "lc_trapezoidal".into(),
            title: "LC ring, trapezoidal (L=1 mH, C=1 µF)".into(),
            kind: "transient".into(),
            notes: "V0=1 V, dt=100 ns, ~1 period".into(),
            t: t.clone(),
            signals: vec![
                Signal {
                    name: "Vc".into(),
                    unit: "V".into(),
                    rust: snaps
                        .iter()
                        .map(|s| s.elm_volts[1][0] - s.elm_volts[1][1])
                        .collect(),
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: snaps.iter().map(|s| s.elm_currents[2]).collect(),
                    java: None,
                },
                Signal {
                    name: "energy".into(),
                    unit: "J".into(),
                    rust: snaps
                        .iter()
                        .map(|s| {
                            let vc = s.elm_volts[1][0] - s.elm_volts[1][1];
                            let i = s.elm_currents[2];
                            0.5 * 1e-6 * vc * vc + 0.5 * 1e-3 * i * i
                        })
                        .collect(),
                    java: None,
                },
            ],
        });
        cases.push(Case {
            id: "lc_backward_euler".into(),
            title: "LC ring, backward Euler".into(),
            kind: "transient".into(),
            notes: "same LC; BE damps".into(),
            t,
            signals: vec![
                Signal {
                    name: "Vc".into(),
                    unit: "V".into(),
                    rust: be_snaps
                        .iter()
                        .map(|s| s.elm_volts[1][0] - s.elm_volts[1][1])
                        .collect(),
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: be_snaps.iter().map(|s| s.elm_currents[2]).collect(),
                    java: None,
                },
                Signal {
                    name: "energy".into(),
                    unit: "J".into(),
                    rust: be_snaps
                        .iter()
                        .map(|s| {
                            let vc = s.elm_volts[1][0] - s.elm_volts[1][1];
                            let i = s.elm_currents[2];
                            0.5 * 1e-6 * vc * vc + 0.5 * 1e-3 * i * i
                        })
                        .collect(),
                    java: None,
                },
            ],
        });
    }

    {
        let mut c = Circuit::new();
        c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 5.0)));
        c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
        c.push(Box::new(Diode::new(100, 0, 100, 100)));
        c.push(Box::new(Wire::new(100, 100, 0, 100)));
        c.dc_operating_point().unwrap();
        let s = c.snapshot();
        let vd = s.elm_volts[2][0] - s.elm_volts[2][1];
        cases.push(Case {
            id: "diode_dc_operating_point".into(),
            title: "Diode DC operating point".into(),
            kind: "dc".into(),
            notes: "5 V — 1 kΩ — default diode".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "Vd".into(),
                    unit: "V".into(),
                    rust: vec![vd],
                    java: None,
                },
                Signal {
                    name: "Iresistor".into(),
                    unit: "A".into(),
                    rust: vec![s.elm_currents[1]],
                    java: None,
                },
                Signal {
                    name: "Idiode".into(),
                    unit: "A".into(),
                    rust: vec![s.elm_currents[2]],
                    java: None,
                },
            ],
        });
    }

    {
        let text = "\
$ 1 0.000005 10 50 5 50
v 0 100 0 0 0 0 40 10 0 0 0.5
r 0 0 100 0 0 1000
r 100 0 100 100 0 1000
w 100 100 0 100 0
";
        let mut c = parse_dump(text).unwrap();
        c.step().unwrap();
        let s = c.snapshot();
        cases.push(Case {
            id: "parse_dump_divider".into(),
            title: "Dump parser divider".into(),
            kind: "dc".into(),
            notes: "CircuitJS1 text dump".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "Vmid".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[1][1]],
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: vec![s.elm_currents[1]],
                    java: None,
                },
            ],
        });
    }

    {
        let text = "\
v 176 256 176 80 0 0 40 5 0 0 0.5
r 176 80 336 80 0 180
c 336 80 336 256 0 1e-6 0 0 0
w 176 256 336 256 0
o 2 64 0 4099 5 0.05 0 2 2 3
";
        let mut c = parse_dump(text).unwrap();
        c.step().unwrap();
        let s = c.snapshot();
        cases.push(Case {
            id: "parse_dump_skips_scopes".into(),
            title: "Dump parser RC (scopes skipped)".into(),
            kind: "dc".into(),
            notes: "one step after load".into(),
            t: vec![s.t],
            signals: vec![
                Signal {
                    name: "Vc".into(),
                    unit: "V".into(),
                    rust: vec![s.elm_volts[2][0] - s.elm_volts[2][1]],
                    java: None,
                },
                Signal {
                    name: "I".into(),
                    unit: "A".into(),
                    rust: vec![s.elm_currents[1]],
                    java: None,
                },
            ],
        });
    }

    let out = format!(
        "{{\"source\":\"circuit-engine compare\",\"caseCount\":{},\"cases\":[{}]}}",
        cases.len(),
        cases
            .iter()
            .map(|c| c.to_json())
            .collect::<Vec<_>>()
            .join(",")
    );
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("compare");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("traces.json");
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(out.as_bytes()).unwrap();
    println!("wrote {} ({} cases)", path.display(), cases.len());
}
