use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::{Ports, FLAG_BACK_EULER};

pub struct Inductor {
    pub ports: Ports,
    pub inductance: f64,
    pub initial_current: f64,
    pub comp_resistance: f64,
    pub cur_source_value: f64,
}

impl Inductor {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, inductance: f64) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), 0),
            inductance,
            initial_current: 0.0,
            comp_resistance: 0.0,
            cur_source_value: 0.0,
        }
    }

    pub fn with_state(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        inductance: f64,
        current: f64,
        initial_current: f64,
    ) -> Self {
        let mut ind = Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            inductance,
            initial_current,
            comp_resistance: 0.0,
            cur_source_value: current,
        };
        ind.ports.current = current;
        ind
    }

    fn is_trapezoidal(&self) -> bool {
        (self.ports.flags & FLAG_BACK_EULER) == 0
    }
}

impl Element for Inductor {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Inductor
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.inductance, "H"))
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
    fn reset(&mut self) {
        self.ports.current = self.initial_current;
        self.cur_source_value = self.initial_current;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        self.comp_resistance = if self.is_trapezoidal() {
            2.0 * self.inductance / ctx.time_step
        } else {
            self.inductance / ctx.time_step
        };
        ctx.stamp_resistor(
            self.ports.nodes[0],
            self.ports.nodes[1],
            self.comp_resistance,
        );
    }
    fn start_iteration(&mut self, _ctx: &mut SimContext) {
        let voltdiff = self.ports.volts[0] - self.ports.volts[1];
        if self.is_trapezoidal() {
            self.cur_source_value = voltdiff / self.comp_resistance + self.ports.current;
        } else {
            self.cur_source_value = self.ports.current;
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        ctx.stamp_current_source(
            self.ports.nodes[0],
            self.ports.nodes[1],
            self.cur_source_value,
        );
    }
    fn calculate_current(&mut self) {
        if self.comp_resistance > 0.0 {
            let voltdiff = self.ports.volts[0] - self.ports.volts[1];
            self.ports.current = voltdiff / self.comp_resistance + self.cur_source_value;
        }
    }
}
