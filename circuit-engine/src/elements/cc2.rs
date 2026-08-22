use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::cc2_posts;
use crate::ports::Ports;

/// Second-generation current conveyor, dump `179`. CCII− uses gain −1.
pub struct Cc2Elm {
    pub ports: Ports,
    pub gain: f64,
    vs: usize,
}

impl Cc2Elm {
    pub fn from_dump(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, gain: f64) -> Self {
        let _ = (x2, y2);
        Self {
            ports: Ports::many(cc2_posts((x1, y1), flags), flags),
            gain,
            vs: 0,
        }
    }
}

impl Element for Cc2Elm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Ccii
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.gain, ""))
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
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (0, self.node(0))
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
        // V(X) = V(Y); I(Z) = gain * I(X)
        ctx.stamp_voltage_source(0, self.ports.nodes[0], self.vs, None);
        ctx.stamp_vs_node(self.vs, self.ports.nodes[1], -1.0);
        ctx.stamp_cccs(0, self.ports.nodes[2], self.vs, self.gain);
    }
}
