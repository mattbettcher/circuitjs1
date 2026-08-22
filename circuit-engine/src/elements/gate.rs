use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::gate_posts;
use crate::ports::Ports;

const FLAG_SCHMITT: i32 = 1 << 1;
const FLAG_INVERT_INPUTS: i32 = 1 << 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateFn {
    And,
    Or,
    Xor,
}

/// Combinational gate, dump 150–154 / 431. Output VS to ground.
pub struct GateElm {
    pub ports: Ports,
    vs: usize,
    op: GateFn,
    inverting: bool,
    input_count: usize,
    high_voltage: f64,
    last_output: bool,
    input_states: Vec<bool>,
}

impl GateElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        op: GateFn,
        inverting: bool,
        input_count: usize,
        last_output_voltage: f64,
        high_voltage: f64,
    ) -> Self {
        let n = input_count.max(1);
        Self {
            ports: Ports::many(gate_posts((x1, y1), (x2, y2), flags, n), flags),
            vs: 0,
            op,
            inverting,
            input_count: n,
            high_voltage,
            last_output: last_output_voltage > high_voltage * 0.5,
            input_states: vec![false; n],
        }
    }

    fn get_input(&mut self, i: usize) -> bool {
        let high = (self.ports.flags & FLAG_INVERT_INPUTS) == 0;
        let v = self.ports.volts[i];
        if (self.ports.flags & FLAG_SCHMITT) == 0 {
            return if v > self.high_voltage * 0.5 {
                high
            } else {
                !high
            };
        }
        let thresh = self.high_voltage * if self.input_states[i] { 0.35 } else { 0.55 };
        let res = v > thresh;
        self.input_states[i] = res;
        if res {
            high
        } else {
            !high
        }
    }

    fn calc_function(&mut self) -> bool {
        match self.op {
            GateFn::And => {
                let mut f = true;
                for i in 0..self.input_count {
                    f &= self.get_input(i);
                }
                f
            }
            GateFn::Or => {
                let mut f = false;
                for i in 0..self.input_count {
                    f |= self.get_input(i);
                }
                f
            }
            GateFn::Xor => {
                let mut f = false;
                for i in 0..self.input_count {
                    f ^= self.get_input(i);
                }
                f
            }
        }
    }
}

impl Element for GateElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        match (self.op, self.inverting) {
            (GateFn::And, false) => ElementKind::AndGate,
            (GateFn::And, true) => ElementKind::NandGate,
            (GateFn::Or, false) => ElementKind::OrGate,
            (GateFn::Or, true) => ElementKind::NorGate,
            (GateFn::Xor, false) => ElementKind::XorGate,
            (GateFn::Xor, true) => ElementKind::XnorGate,
        }
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        n == self.input_count
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (0, self.node(self.input_count))
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
    }
    fn set_voltage_source(&mut self, _n: usize, vs: usize) {
        self.vs = vs;
    }
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        self.ports.set_voltage(n, v);
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ports.current
    }
    fn set_current(&mut self, _vs: usize, c: f64) {
        self.ports.current = c;
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        ctx.stamp_voltage_source(0, self.ports.nodes[self.input_count], self.vs, None);
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let mut f = self.calc_function();
        if self.inverting {
            f = !f;
        }
        self.last_output = f;
        let res = if self.last_output {
            self.high_voltage
        } else {
            0.0
        };
        ctx.update_voltage_source(self.vs, res);
    }
}
