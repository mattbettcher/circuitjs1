use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

pub const FLAG_ESCAPE: i32 = 4;

/// Named net. Posts with the same text are unioned during analyze.
pub struct LabeledNode {
    pub ports: Ports,
    pub text: String,
}

impl LabeledNode {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, text: impl Into<String>) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), FLAG_ESCAPE),
            text: text.into(),
        }
    }

    pub fn from_dump(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, text: String) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            text,
        }
    }
}

impl Element for LabeledNode {
    fn post_count(&self) -> usize {
        1
    }
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts[..1]
    }
    fn geometry(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::LabeledNode
    }
    fn tag(&self) -> &'static str {
        "label"
    }
    fn is_removable_wire(&self) -> bool {
        true
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 == n2
    }
    fn net_name(&self) -> Option<&str> {
        Some(&self.text)
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
