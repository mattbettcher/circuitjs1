use crate::context::SimContext;
use crate::element::Element;
use crate::ports::Ports;

pub struct CurrentElm {
    pub ports: Ports,
    pub current_value: f64,
}

impl CurrentElm {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, current: f64) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), 0),
            current_value: current,
        }
    }
}

impl Element for CurrentElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
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
    fn stamp(&mut self, ctx: &mut SimContext) {
        ctx.stamp_current_source(self.ports.nodes[0], self.ports.nodes[1], self.current_value);
        self.ports.current = self.current_value;
    }
}
