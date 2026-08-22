use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

/// Schmitt trigger, dump `182` (non-inverting) / `183` (inverting).
pub struct SchmittElm {
    pub ports: Ports,
    vs: usize,
    inverting: bool,
    slew_rate: f64,
    lower: f64,
    upper: f64,
    logic_on: f64,
    logic_off: f64,
    state: bool,
    last_out: f64,
}

impl SchmittElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        inverting: bool,
        slew_rate: f64,
        lower: f64,
        upper: f64,
        logic_on: f64,
        logic_off: f64,
    ) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            vs: 0,
            inverting,
            slew_rate,
            lower,
            upper,
            logic_on,
            logic_off,
            state: false,
            last_out: 0.0,
        }
    }
}

impl Element for SchmittElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        if self.inverting {
            ElementKind::InvertingSchmitt
        } else {
            ElementKind::Schmitt
        }
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        n == 1
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (0, self.node(1))
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
        ctx.stamp_voltage_source(0, self.ports.nodes[1], self.vs, None);
    }
    fn start_iteration(&mut self, _ctx: &mut SimContext) {
        self.last_out = self.ports.volts[1];
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let vin = self.ports.volts[0];
        let mut out;
        if self.inverting {
            if self.state {
                if vin > self.upper {
                    self.state = false;
                    out = self.logic_off;
                } else {
                    out = self.logic_on;
                }
            } else if vin < self.lower {
                self.state = true;
                out = self.logic_on;
            } else {
                out = self.logic_off;
            }
        } else if self.state {
            if vin > self.upper {
                self.state = false;
                out = self.logic_on;
            } else {
                out = self.logic_off;
            }
        } else if vin < self.lower {
            self.state = true;
            out = self.logic_off;
        } else {
            out = self.logic_on;
        }
        let max_step = self.slew_rate * ctx.time_step * 1e9;
        let v0 = if self.inverting {
            self.ports.volts[1]
        } else {
            self.last_out
        };
        out = out.clamp(v0 - max_step, v0 + max_step);
        ctx.update_voltage_source(self.vs, out);
    }
}
