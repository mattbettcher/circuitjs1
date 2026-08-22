use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{chip_pin_post, SIDE_E, SIDE_W};
use crate::ports::Ports;

const FLAG_RESET: i32 = 2;
const FLAG_SET: i32 = 4;
const FLAG_INVERT_SET_RESET: i32 = 8;

#[derive(Clone, Debug)]
struct ChipPin {
    pos: i32,
    side: i32,
    output: bool,
    #[allow(dead_code)]
    state: bool,
    value: bool,
    vs: usize,
    current: f64,
}

/// D flip-flop, dump `155`. ChipElm digital rails: each output is a VS to ground.
pub struct DFlipFlopElm {
    ports: Ports,
    pins: Vec<ChipPin>,
    high_voltage: f64,
    last_clock: bool,
    just_loaded: bool,
}

impl DFlipFlopElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        high_voltage: f64,
        q_volts: f64,
    ) -> Self {
        let _ = (x2, y2);
        let has_set = (flags & FLAG_SET) != 0;
        let has_reset = (flags & FLAG_RESET) != 0 || has_set;
        let mut pins = vec![
            ChipPin {
                pos: 0,
                side: SIDE_W,
                output: false,
                state: false,
                value: false,
                vs: 0,
                current: 0.0,
            },
            ChipPin {
                pos: 0,
                side: SIDE_E,
                output: true,
                state: true,
                value: q_volts > high_voltage * 0.5,
                vs: 0,
                current: 0.0,
            },
            ChipPin {
                pos: if has_set { 1 } else { 2 },
                side: SIDE_E,
                output: true,
                state: false,
                value: !(q_volts > high_voltage * 0.5),
                vs: 0,
                current: 0.0,
            },
            ChipPin {
                pos: 1,
                side: SIDE_W,
                output: false,
                state: false,
                value: false,
                vs: 0,
                current: 0.0,
            },
        ];
        if has_set {
            pins.push(ChipPin {
                pos: 2,
                side: SIDE_E,
                output: false,
                state: false,
                value: false,
                vs: 0,
                current: 0.0,
            });
            pins.push(ChipPin {
                pos: 2,
                side: SIDE_W,
                output: false,
                state: false,
                value: false,
                vs: 0,
                current: 0.0,
            });
        } else if has_reset {
            pins.push(ChipPin {
                pos: 2,
                side: SIDE_W,
                output: false,
                state: false,
                value: false,
                vs: 0,
                current: 0.0,
            });
        }
        let posts: Vec<(i32, i32)> = pins
            .iter()
            .map(|p| chip_pin_post((x1, y1), flags, 2, 3, p.pos, p.side))
            .collect();
        let mut ports = Ports::many(posts, flags);
        if ports.volts.len() > 1 {
            ports.volts[1] = q_volts;
        }
        Self {
            ports,
            pins,
            high_voltage,
            last_clock: false,
            just_loaded: true,
        }
    }

    fn has_set(&self) -> bool {
        (self.ports.flags & FLAG_SET) != 0
    }
    fn has_reset(&self) -> bool {
        (self.ports.flags & FLAG_RESET) != 0 || self.has_set()
    }
    fn invert_set_reset(&self) -> bool {
        (self.ports.flags & FLAG_INVERT_SET_RESET) != 0
    }

    fn write_output(&mut self, n: usize, value: bool) {
        if n < self.pins.len() {
            self.pins[n].value = value;
        }
    }

    fn execute(&mut self) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let invert = self.invert_set_reset();
        let is_set = self.has_set() && self.pins.get(5).is_some_and(|p| p.value != invert);
        let is_reset = self.has_reset() && self.pins.get(4).is_some_and(|p| p.value != invert);
        if is_set || is_reset {
            self.write_output(1, false);
            self.write_output(2, false);
            if is_set {
                self.write_output(1, true);
            }
            if is_reset {
                self.write_output(2, true);
            }
        } else {
            if self.pins[3].value && !self.last_clock {
                let d = self.pins[0].value;
                self.write_output(1, d);
            }
            let q = self.pins[1].value;
            self.write_output(2, !q);
        }
        self.last_clock = self.pins[3].value;
    }
}

impl Element for DFlipFlopElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::DFlipFlop
    }
    fn voltage_source_count(&self) -> usize {
        2
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        self.pins.get(n).is_some_and(|p| p.output)
    }
    fn vs_nodes(&self, local: usize) -> (usize, usize) {
        let mut k = 0;
        for (i, pin) in self.pins.iter().enumerate() {
            if pin.output {
                if k == local {
                    return (0, self.node(i));
                }
                k += 1;
            }
        }
        (0, 0)
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
    }
    fn set_voltage_source(&mut self, n: usize, vs: usize) {
        let mut k = 0;
        for pin in &mut self.pins {
            if pin.output {
                if k == n {
                    pin.vs = vs;
                    return;
                }
                k += 1;
            }
        }
    }
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        self.ports.set_voltage(n, v);
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.pins.get(1).map(|p| p.current).unwrap_or(0.0)
    }
    fn set_current(&mut self, local: usize, c: f64) {
        let mut k = 0;
        for pin in &mut self.pins {
            if pin.output {
                if k == local {
                    pin.current = c;
                    if local == 0 {
                        self.ports.current = c;
                    }
                    return;
                }
                k += 1;
            }
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        for (i, pin) in self.pins.iter().enumerate() {
            if pin.output {
                ctx.stamp_voltage_source(0, self.ports.nodes[i], pin.vs, None);
            }
        }
    }
    fn start_iteration(&mut self, _ctx: &mut SimContext) {
        let thr = self.high_voltage * 0.5;
        for i in 0..self.pins.len() {
            if !self.pins[i].output {
                self.pins[i].value = self.ports.volts[i] > thr;
            }
        }
        self.execute();
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        for pin in &self.pins {
            if pin.output {
                ctx.update_voltage_source(pin.vs, if pin.value { self.high_voltage } else { 0.0 });
            }
        }
    }
    fn reset(&mut self) {
        for pin in &mut self.pins {
            pin.value = false;
            pin.current = 0.0;
        }
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
        if self.ports.volts.len() > 2 {
            self.ports.volts[2] = self.high_voltage;
        }
        if self.pins.len() > 2 {
            self.pins[2].value = true;
        }
        self.last_clock = false;
        self.just_loaded = false;
    }
}
