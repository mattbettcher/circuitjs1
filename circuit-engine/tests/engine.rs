use circuit_engine::elements::{
    Capacitor, CurrentElm, Diode, Ground, Inductor, Resistor, VoltageElm, Wire,
};
use circuit_engine::{parse_dump, Circuit, FLAG_BACK_EULER};

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
