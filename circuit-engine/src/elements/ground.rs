use crate::context::SimContext;
use crate::element::Element;
use crate::ports::Ports;

pub struct Ground {
    pub ports: Ports,
}

impl Ground {
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

impl Element for Ground {
    fn post_count(&self) -> usize {
        1
    }
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts[..1]
    }
    fn is_removable_wire(&self) -> bool {
        true
    }
    fn is_ground(&self) -> bool {
        true
    }
    fn has_ground_connection(&self, _post: usize) -> bool {
        true
    }
    fn connected_post(&self, _post: usize) -> Option<(i32, i32)> {
        // Topology merges all grounds; returning None is handled by Circuit.
        None
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes.get(post).copied().unwrap_or(0)
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
    fn stamp(&mut self, _ctx: &mut SimContext) {}
}
