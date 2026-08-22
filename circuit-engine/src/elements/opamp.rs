use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{dsign, interp2};
use crate::ports::Ports;

const FLAG_SWAP: i32 = 1;
const FLAG_SMALL: i32 = 2;
const FLAG_LOWGAIN: i32 = 4;
const FLAG_GAIN: i32 = 8;

pub struct OpAmpElm {
    pub ports: Ports,
    pub max_out: f64,
    pub min_out: f64,
    pub gain: f64,
    vs: usize,
    last_vd: f64,
}

impl OpAmpElm {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::from_dump(x1, y1, x2, y2, FLAG_GAIN, 15.0, -15.0, 1e6, 1e5)
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        max_out: f64,
        min_out: f64,
        _gbw: f64,
        gain: f64,
    ) -> Self {
        let opheight = if (flags & FLAG_SMALL) != 0 { 8 } else { 16 };
        let mut hs = opheight * dsign((x1, y1), (x2, y2));
        if (flags & FLAG_SWAP) != 0 {
            hs = -hs;
        }
        let (inn, inp) = interp2((x1, y1), (x2, y2), 0.0, hs as f64);
        let mut g = gain;
        if (flags & FLAG_GAIN) == 0 {
            g = if (flags & FLAG_LOWGAIN) != 0 {
                1000.0
            } else {
                100_000.0
            };
        }
        Self {
            ports: Ports::many(vec![inn, inp, (x2, y2)], flags),
            max_out,
            min_out,
            gain: g,
            vs: 0,
            last_vd: 0.0,
        }
    }
}

impl Element for OpAmpElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::OpAmp
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.gain, ""))
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn has_ground_connection(&self, post: usize) -> bool {
        post == 2
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (0, self.node(2))
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
        ctx.stamp_node_vs(self.ports.nodes[2], self.vs, 1.0);
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let vd = self.ports.volts[1] - self.ports.volts[0];
        let midpoint = (self.max_out + self.min_out) * 0.5;
        if (self.last_vd - vd).abs() > 0.1 {
            ctx.converged = false;
        } else if self.ports.volts[2] > self.max_out + 0.1
            || self.ports.volts[2] < self.min_out - 0.1
        {
            ctx.converged = false;
        }
        let max_adj = self.max_out - midpoint;
        let min_adj = self.min_out - midpoint;
        let (dx, x) = if vd >= max_adj / self.gain && self.last_vd >= 0.0 {
            let dx = 1e-4;
            (dx, self.max_out - dx * max_adj / self.gain)
        } else if vd <= min_adj / self.gain && self.last_vd <= 0.0 {
            let dx = 1e-4;
            (dx, self.min_out - dx * min_adj / self.gain)
        } else {
            (self.gain, midpoint)
        };
        ctx.stamp_vs_node(self.vs, self.ports.nodes[0], dx);
        ctx.stamp_vs_node(self.vs, self.ports.nodes[1], -dx);
        ctx.stamp_vs_node(self.vs, self.ports.nodes[2], 1.0);
        ctx.stamp_right_side_vs(self.vs, x);
        self.last_vd = vd;
    }
}
