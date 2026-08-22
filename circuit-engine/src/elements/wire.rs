use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

pub struct Wire {
    pub ports: Ports,
}

impl Wire {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), 0),
        }
    }

    pub fn with_flags(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
        }
    }
}

impl Element for Wire {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Wire
    }
    fn is_removable_wire(&self) -> bool {
        true
    }
    fn connected_post(&self, post: usize) -> Option<(i32, i32)> {
        if post == 0 {
            Some(self.ports.posts[1])
        } else {
            Some(self.ports.posts[0])
        }
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
    fn set_current(&mut self, _vs: usize, c: f64) {
        self.ports.current = c;
    }
    fn stamp(&mut self, _ctx: &mut SimContext) {}
}
