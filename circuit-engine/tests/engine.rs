use circuit_engine::elements::{
    Capacitor, CurrentElm, Diode, Ground, Inductor, Resistor, VoltageElm, Wire,
};
use circuit_engine::{parse_dump, Circuit, Element, FLAG_BACK_EULER};

fn divider() -> Circuit {
    let mut c = Circuit::new();
    c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 10.0)));
    c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
    c.push(Box::new(Resistor::new(100, 0, 100, 100, 1000.0)));
    c.push(Box::new(Wire::new(100, 100, 0, 100)));
    c
}

#[test]
fn voltage_divider_dc() {
    let mut c = divider();
    c.step().unwrap();
    let top = c.element_volts(1);
    let bot = c.element_volts(2);
    assert!((top[0] - 10.0).abs() < 1e-9, "source+ = {}", top[0]);
    assert!((top[1] - 5.0).abs() < 1e-9, "mid = {}", top[1]);
    assert!((bot[0] - 5.0).abs() < 1e-9);
    assert!(bot[1].abs() < 1e-9);
    assert!((c.element_current(1) - 0.005).abs() < 1e-12);
}

#[test]
fn current_source_and_load() {
    let mut c = Circuit::new();
    c.push(Box::new(CurrentElm::new(0, 100, 0, 0, 0.002)));
    c.push(Box::new(Resistor::new(0, 0, 0, 100, 2500.0)));
    c.push(Box::new(Ground::new(0, 100, 0, 116)));
    c.step().unwrap();
    let v = c.element_volts(1);
    assert!((v[0] - 5.0).abs() < 1e-9, "v0={} v1={}", v[0], v[1]);
    assert!(v[1].abs() < 1e-9);
}

#[test]
fn wire_merges_nodes() {
    let mut c = divider();
    c.analyze().unwrap();
    // ground + source+ + mid = 3 nodes (indices 0..=2)
    assert_eq!(c.node_count(), 3);
}

#[test]
fn unconnected_resistor_does_not_singular() {
    let mut c = Circuit::new();
    c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
    c.step().unwrap();
    let v = c.element_volts(0);
    assert!(v[0].abs() < 1e-6);
    assert!(v[1].abs() < 1e-6);
}

#[test]
fn two_grounded_islands() {
    let mut c = Circuit::new();
    c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 10.0)));
    c.push(Box::new(Resistor::new(0, 0, 0, 100, 1000.0)));
    c.push(Box::new(VoltageElm::dc(200, 100, 200, 0, 3.0)));
    c.push(Box::new(Resistor::new(200, 0, 200, 100, 1000.0)));
    c.push(Box::new(Ground::new(0, 100, 0, 116)));
    c.push(Box::new(Ground::new(200, 100, 200, 116)));
    c.analyze().unwrap();
    assert_eq!(c.island_count(), 2);
    c.step().unwrap();
    assert!((c.element_volts(1)[0] - 10.0).abs() < 1e-9);
    assert!((c.element_volts(3)[0] - 3.0).abs() < 1e-9);
}

#[test]
fn rc_step_response() {
    let mut c = Circuit::new();
    c.set_time_step(1e-6);
    c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 1.0)));
    c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
    let mut cap = Capacitor::new(100, 0, 100, 100, 1e-6);
    cap.voltdiff = 0.0;
    cap.initial_voltage = 0.0;
    c.push(Box::new(cap));
    c.push(Box::new(Wire::new(100, 100, 0, 100)));
    c.run_to(1e-3).unwrap();
    let vc = c.element_volts(2)[0] - c.element_volts(2)[1];
    let expected = 1.0 - (-1.0f64).exp();
    assert!(
        (vc - expected).abs() < 0.002,
        "Vc={vc} expected={expected} t={}",
        c.t()
    );
}

#[test]
fn lc_energy_trapezoidal_vs_backward_euler() {
    fn energy(c: &Circuit) -> f64 {
        let vc = c.element_volts(1)[0] - c.element_volts(1)[1];
        let i = c.element_current(2);
        0.5 * 1e-6 * vc * vc + 0.5 * 1e-3 * i * i
    }

    fn lc(flags: i32) -> Circuit {
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

    let e0 = 0.5 * 1e-6;
    let mut trap = lc(0);
    trap.steps(2000).unwrap();
    let e_trap = energy(&trap);

    let mut be = lc(FLAG_BACK_EULER);
    be.steps(2000).unwrap();
    let e_be = energy(&be);

    assert!((e_trap - e0).abs() < 1e-10, "trap energy {e_trap} vs {e0}");
    assert!(
        (e_be - e0).abs() < 0.05 * e0,
        "BE should damp; energy {e_be} vs {e0}"
    );
    assert!(
        (e_trap - e0).abs() < (e_be - e0).abs(),
        "trapezoidal should conserve better than BE"
    );
}

#[test]
fn diode_dc_operating_point() {
    let mut c = Circuit::new();
    c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 5.0)));
    c.push(Box::new(Resistor::new(0, 0, 100, 0, 1000.0)));
    c.push(Box::new(Diode::new(100, 0, 100, 100)));
    c.push(Box::new(Wire::new(100, 100, 0, 100)));
    c.dc_operating_point().unwrap();

    let vd = c.element_volts(2)[0] - c.element_volts(2)[1];
    let i_d = c.element_current(2);
    let i_r = c.element_current(1);
    assert!(vd > 0.5 && vd < 0.95, "Vd={vd}");
    assert!(i_r > 0.004 && i_r < 0.005, "i_r={i_r}");
    assert!((i_d - i_r).abs() / i_r.abs() < 0.02, "i_d={i_d} i_r={i_r}");
    assert!((i_r - (5.0 - vd) / 1000.0).abs() < 1e-9);
}

#[test]
fn parse_dump_divider() {
    let text = "\
$ 1 0.000005 10 50 5 50
v 0 100 0 0 0 0 40 10 0 0 0.5
r 0 0 100 0 0 1000
r 100 0 100 100 0 1000
w 100 100 0 100 0
";
    let mut c = parse_dump(text).unwrap();
    assert!((c.time_step() - 5e-6).abs() < 1e-18);
    c.step().unwrap();
    let mid = c.element_volts(1)[1];
    assert!((mid - 5.0).abs() < 1e-9, "mid={mid}");
}

#[test]
fn parse_dump_skips_scopes() {
    let text = "\
v 176 256 176 80 0 0 40 5 0 0 0.5
r 176 80 336 80 0 180
c 336 80 336 256 0 1e-6 0 0 0
w 176 256 336 256 0
o 2 64 0 4099 5 0.05 0 2 2 3
& 2 0 0.000001 0.000101 Capacitance
";
    let mut c = parse_dump(text).unwrap();
    c.step().unwrap();
    assert!(c.node_count() >= 2);
}

#[test]
fn closed_switch_is_wire() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
s 100 0 100 100 0 0 false
w 100 100 0 100 0
",
    )
    .unwrap();
    c.step().unwrap();
    assert!((c.element_current(1) - 0.005).abs() < 1e-9);
}

#[test]
fn open_switch_isolates() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
s 100 0 100 100 0 1 false
w 100 100 0 100 0
",
    )
    .unwrap();
    c.step().unwrap();
    assert!(c.element_current(1).abs() < 1e-8);
}

#[test]
fn switch_toggle_reanalyzes() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
s 100 0 100 100 0 0 false
w 100 100 0 100 0
",
    )
    .unwrap();
    c.step().unwrap();
    c.toggle(2);
    c.step().unwrap();
    assert!(c.element_current(1).abs() < 1e-8);
}

#[test]
fn rail_five_volts() {
    let mut c = parse_dump(
        "\
R 0 0 0 16 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
g 100 0 100 16 0
",
    )
    .unwrap();
    c.step().unwrap();
    assert!((c.element_volts(1)[0] - 5.0).abs() < 1e-9);
    assert!(c.element_volts(1)[1].abs() < 1e-9);
}

#[test]
fn labeled_nodes_merge() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
207 100 0 116 0 4 mid
207 100 100 116 100 4 mid
w 100 100 0 100 0
",
    )
    .unwrap();
    c.step().unwrap();
    assert!((c.element_current(1) - 0.005).abs() < 1e-9);
}

#[test]
fn pot_center_divides() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 10 0 0 0.5
g 0 100 0 116 0
174 0 0 100 0 0 1000 0.5 Resistance
g 100 0 100 16 0
",
    )
    .unwrap();
    c.step().unwrap();
    let v = c.element_volts(2);
    assert!((v[0] - 10.0).abs() < 1e-6, "hot={}", v[0]);
    assert!(v[1].abs() < 1e-6, "cold={}", v[1]);
    assert!((v[2] - 5.0).abs() < 1e-3, "wiper={}", v[2]);
}

#[test]
fn opamp_follower() {
    use circuit_engine::elements::OpAmpElm;
    let mut c = Circuit::new();
    let op = OpAmpElm::new(48, 16, 80, 16);
    let inn = op.posts()[0];
    let inp = op.posts()[1];
    let out = op.posts()[2];
    c.push(Box::new(VoltageElm::dc(
        inp.0,
        inp.1 + 32,
        inp.0,
        inp.1,
        2.0,
    )));
    c.push(Box::new(Ground::new(inp.0, inp.1 + 32, inp.0, inp.1 + 48)));
    c.push(Box::new(op));
    c.push(Box::new(Wire::new(out.0, out.1, inn.0, inn.1)));
    c.step().unwrap();
    let vout = c.element_volts(2)[2];
    assert!((vout - 2.0).abs() < 0.05, "follower out={vout}");
}

#[test]
fn npn_ce_vbe() {
    use circuit_engine::elements::TransistorElm;
    let mut c = Circuit::new();
    let q = TransistorElm::npn(16, 32, 48, 32);
    let base = q.posts()[0];
    let coll = q.posts()[1];
    let emit = q.posts()[2];
    c.push(Box::new(VoltageElm::dc(0, 100, 0, 0, 5.0)));
    c.push(Box::new(Resistor::new(0, 0, coll.0, coll.1, 1000.0)));
    c.push(Box::new(Resistor::new(0, 0, base.0, base.1, 100_000.0)));
    c.push(Box::new(q));
    c.push(Box::new(Ground::new(emit.0, emit.1, emit.0, emit.1 + 16)));
    c.push(Box::new(Wire::new(0, 100, emit.0, emit.1)));
    c.step().unwrap();
    let v = c.element_volts(3);
    let vbe = v[0] - v[2];
    assert!(vbe > 0.5 && vbe < 0.9, "Vbe={vbe}");
}

#[test]
fn nmos_linear_ids() {
    use circuit_engine::elements::MosfetElm;
    let mut c = Circuit::new();
    let m = MosfetElm::nmos(16, 32, 48, 32);
    let g = m.posts()[0];
    let s = m.posts()[1];
    let d = m.posts()[2];
    c.push(Box::new(VoltageElm::dc(g.0, g.1 + 48, g.0, g.1, 3.0)));
    c.push(Box::new(VoltageElm::dc(d.0, d.1 + 48, d.0, d.1, 1.0)));
    c.push(Box::new(m));
    c.push(Box::new(Ground::new(s.0, s.1, s.0, s.1 + 16)));
    c.push(Box::new(Wire::new(g.0, g.1 + 48, s.0, s.1)));
    c.push(Box::new(Wire::new(d.0, d.1 + 48, s.0, s.1)));
    c.step().unwrap();
    let ids = c.element_current(2);
    // beta=0.02, vt=1.5, vgs=3, vds=1 → linear: 0.02*((1.5)*1 - 0.5) = 0.02
    assert!((ids - 0.02).abs() < 0.005, "Ids={ids}");
}

#[test]
fn linear_vccs() {
    use circuit_engine::elements::VccsElm;
    let mut c = Circuit::new();
    let src = VccsElm::new(0, 0, 64, 32, 0, 0.001);
    let a = src.posts()[0];
    let b = src.posts()[1];
    let cp = src.posts()[2];
    let cm = src.posts()[3];
    c.push(Box::new(VoltageElm::dc(a.0, a.1 + 16, a.0, a.1, 1.0)));
    c.push(Box::new(Ground::new(b.0, b.1, b.0, b.1 + 16)));
    c.push(Box::new(Wire::new(a.0, a.1 + 16, b.0, b.1)));
    c.push(Box::new(src));
    c.push(Box::new(Resistor::new(cp.0, cp.1, cm.0, cm.1, 1000.0)));
    c.step().unwrap();
    let i = c.element_current(3);
    assert!((i - 0.001).abs() < 1e-6, "I={i}");
}

#[test]
fn spdt_routes_common() {
    use circuit_engine::elements::Switch2Elm;
    let sw = Switch2Elm::new(0, 0, 48, 0);
    let t0 = sw.posts()[1];
    let t1 = sw.posts()[2];
    let mut c = Circuit::new();
    c.push(Box::new(VoltageElm::dc(0, 16, 0, 0, 5.0)));
    c.push(Box::new(Ground::new(0, 16, 0, 32)));
    c.push(Box::new(sw));
    c.push(Box::new(Resistor::new(t0.0, t0.1, t0.0, t0.1 + 16, 1000.0)));
    c.push(Box::new(Ground::new(t0.0, t0.1 + 16, t0.0, t0.1 + 32)));
    c.push(Box::new(Resistor::new(t1.0, t1.1, t1.0, t1.1 + 16, 1000.0)));
    c.push(Box::new(Ground::new(t1.0, t1.1 + 16, t1.0, t1.1 + 32)));
    c.step().unwrap();
    assert!(
        (c.element_current(3) - 0.005).abs() < 1e-6,
        "throw0 I={}",
        c.element_current(3)
    );
    assert!(
        c.element_current(5).abs() < 1e-8,
        "throw1 I={}",
        c.element_current(5)
    );
    c.toggle(2);
    c.step().unwrap();
    assert!(c.element_current(3).abs() < 1e-8);
    assert!((c.element_current(5) - 0.005).abs() < 1e-6);
}

#[test]
fn zener_shunt_breakdown() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 10 0 0 0.5
r 0 0 100 0 0 1000
z 100 100 100 0 0 5.6
w 100 100 0 100 0
",
    )
    .unwrap();
    c.step().unwrap();
    let vz = c.element_volts(2)[1] - c.element_volts(2)[0];
    assert!(vz > 5.0 && vz < 6.5, "Vz reverse={vz}");
}

#[test]
fn analog_switch_closed_conducts() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
159 100 0 100 100 0 20 10000000000 2.5
w 100 100 0 100 0
R 84 50 84 34 0 0 40 5 0 0 0.5
g 0 100 0 116 0
",
    )
    .unwrap();
    c.step().unwrap();
    let i = c.element_current(1);
    assert!((i - 5.0 / 1020.0).abs() < 1e-6, "I={i}");
}

#[test]
fn transformer_loads_without_nan() {
    let mut c = parse_dump(
        "\
$ 0 0.000001
v 0 64 0 16 0 1 40 5 0 0 0.5
r 0 16 64 16 0 100
T 64 16 128 48 0 4 1 0 0 0.999
r 64 48 0 48 0 100
g 0 64 0 80 0
r 128 16 192 16 0 1000
w 128 48 192 16 0
g 192 16 192 32 0
",
    )
    .unwrap();
    for _ in 0..20 {
        c.step().unwrap();
    }
    let v = c.element_volts(2);
    assert!(v.iter().all(|x| x.is_finite()), "{v:?}");
}

#[test]
fn inverting_schmitt_low_in_is_high_out() {
    let mut c = parse_dump(
        "\
R 0 0 0 16 0 0 40 0 0 0 0.5
g 0 16 0 32 0
183 0 0 64 0 0 0.5 1.66 3.33 5 0
",
    )
    .unwrap();
    c.step().unwrap();
    let vo = c.element_volts(2)[1];
    assert!((vo - 5.0).abs() < 0.05, "Vo={vo}");
}

#[test]
fn jfet_dump_parses() {
    let mut c = parse_dump(
        "\
v 0 64 0 16 0 0 40 5 0 0 0.5
r 0 16 48 16 0 1000
j 48 16 80 16 0 -4 0.00125
g 80 0 80 16 0
g 0 64 0 80 0
",
    )
    .unwrap();
    c.step().unwrap();
    assert!(c.element_volts(2).iter().all(|x| x.is_finite()));
}

#[test]
fn ccvs_linear_gain() {
    let mut c = parse_dump(
        "\
v 0 48 0 16 0 0 40 5 0 0 0.5
r 0 16 48 16 0 5000
214 48 16 80 16 0 2 1000*a
g 0 48 0 64 0
g 48 48 48 64 0
g 144 48 144 64 0
",
    )
    .unwrap();
    c.step().unwrap();
    let vout = c.element_volts(2)[2] - c.element_volts(2)[3];
    assert!((vout - 1.0).abs() < 0.02, "Vout={vout}");
}

#[test]
fn inverter_inverts_logic_input() {
    let mut c = parse_dump(
        "\
g 0 32 0 48 0
L 0 0 16 0 0 1 false 5 0
I 0 0 64 0 0 0.5 5
",
    )
    .unwrap();
    c.step().unwrap();
    c.step().unwrap();
    let v = c.element_volts(2);
    assert!((v[0] - 5.0).abs() < 0.02, "Vin={}", v[0]);
    assert!(v[1].abs() < 0.02, "Vout={}", v[1]);
}

#[test]
fn and_gate_both_high() {
    let mut c = parse_dump(
        "\
g 0 80 0 96 0
L 32 16 16 16 0 1 false 5 0
L 32 48 16 48 0 1 false 5 0
150 32 32 96 32 0 2 0 5
",
    )
    .unwrap();
    c.step().unwrap();
    c.step().unwrap();
    let v = c.element_volts(3);
    assert!((v[2] - 5.0).abs() < 0.02, "Vout={} volts={v:?}", v[2]);
}

#[test]
fn nand_gate_one_low() {
    let mut c = parse_dump(
        "\
g 0 80 0 96 0
L 32 16 16 16 0 1 false 5 0
L 32 48 16 48 0 0 false 5 0
151 32 32 96 32 0 2 0 5
",
    )
    .unwrap();
    c.step().unwrap();
    c.step().unwrap();
    let v = c.element_volts(3);
    assert!((v[2] - 5.0).abs() < 0.02, "Vout={} volts={v:?}", v[2]);
}

#[test]
fn d_flip_flop_rising_edge() {
    let mut c = parse_dump(
        "\
g 160 80 160 96 0
L 0 0 16 0 0 1 false 5 0
L 0 32 16 32 0 0 false 5 0
155 0 0 64 48 0
",
    )
    .unwrap();
    c.step().unwrap();
    let q0 = c.element_volts(3)[1];
    assert!(q0.abs() < 0.5, "Q before clock={q0}");
    c.toggle(2);
    c.step().unwrap();
    c.step().unwrap();
    let q1 = c.element_volts(3)[1];
    assert!((q1 - 5.0).abs() < 0.02, "Q after rising edge={q1}");
}

#[test]
fn fuse_and_ldr_parse() {
    let mut c = parse_dump(
        "\
v 0 100 0 0 0 0 40 1 0 0 0.5
404 0 0 100 0 0 10 100 0 false
374 100 0 100 100 0 0.34 Light
w 100 100 0 100 0
",
    )
    .unwrap();
    c.step().unwrap();
    assert!(c.element_current(1).abs() > 0.0);
}
