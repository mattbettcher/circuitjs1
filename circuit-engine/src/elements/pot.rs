use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::pot_wiper;
use crate::ports::Ports;

pub struct PotElm {
    pub ports: Ports,
    pub max_resistance: f64,
    pub position: f64,
    pub slider_text: String,
}

impl PotElm {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, max_r: f64, position: f64) -> Self {
        Self::from_dump(x1, y1, x2, y2, 0, max_r, position, "Resistance".into())
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        max_resistance: f64,
        position: f64,
        slider_text: String,
    ) -> Self {
        let wiper = pot_wiper((x1, y1), (x2, y2), flags);
        Self {
            ports: Ports::many(vec![(x1, y1), (x2, y2), wiper], flags),
            max_resistance,
            position: position.clamp(0.0, 1.0),
            slider_text,
        }
    }

    fn split(&self) -> (f64, f64) {
        let p = self.position.clamp(1e-6, 1.0 - 1e-6);
        (self.max_resistance * p, self.max_resistance * (1.0 - p))
    }
}

impl Element for PotElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Pot
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.max_resistance, "Ω"))
    }
    fn set_slider(&mut self, t: f64) {
        self.position = t.clamp(0.0, 1.0);
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
        let (r1, r2) = self.split();
        ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[2], r1);
        ctx.stamp_resistor(self.ports.nodes[2], self.ports.nodes[1], r2);
    }
    fn calculate_current(&mut self) {
        let (r1, r2) = self.split();
        let i1 = (self.ports.volts[0] - self.ports.volts[2]) / r1;
        let i2 = (self.ports.volts[1] - self.ports.volts[2]) / r2;
        self.ports.current = i1;
        let _ = i2;
    }
}
