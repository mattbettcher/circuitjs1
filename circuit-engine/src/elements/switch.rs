use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

/// SPST switch. Position 0 = closed, 1 = open.
pub struct SwitchElm {
    pub ports: Ports,
    pub position: i32,
    pub pos_count: i32,
    pub momentary: bool,
    pub resistance: f64,
    pub label: Option<String>,
}

impl SwitchElm {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, closed: bool) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), 0),
            position: if closed { 0 } else { 1 },
            pos_count: 2,
            momentary: false,
            resistance: 0.0,
            label: None,
        }
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        position: i32,
        momentary: bool,
        label: Option<String>,
    ) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            position,
            pos_count: 2,
            momentary,
            resistance: 0.0,
            label,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.position == 0
    }
}

impl Element for SwitchElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Switch
    }
    fn tag(&self) -> &'static str {
        if self.position == 0 {
            "closed"
        } else {
            "open"
        }
    }
    fn is_removable_wire(&self) -> bool {
        self.position == 0 && self.resistance == 0.0
    }
    fn connected_post(&self, post: usize) -> Option<(i32, i32)> {
        if !self.is_removable_wire() {
            return None;
        }
        Some(self.ports.posts[1 - post])
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        self.position == 0
    }
    fn toggle(&mut self) -> bool {
        self.position += 1;
        if self.position >= self.pos_count {
            self.position = 0;
        }
        true
    }
    fn position(&self) -> i32 {
        self.position
    }
    fn set_position(&mut self, p: i32) {
        self.position = p.clamp(0, self.pos_count - 1);
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
    }
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        self.ports.set_voltage(n, v);
        self.calculate_current();
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ports.current
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        if self.position == 0 && self.resistance > 0.0 {
            ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], self.resistance);
        }
    }
    fn calculate_current(&mut self) {
        if self.position == 1 {
            self.ports.current = 0.0;
        } else if self.resistance > 0.0 {
            self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / self.resistance;
        }
    }
}
