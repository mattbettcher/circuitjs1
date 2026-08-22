use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::elements::diode::JunctionDiode;
use crate::ports::{Ports, FLAG_BACK_EULER};

/// Varactor = junction diode + voltage-dependent C, dump `176`.
pub struct VaractorElm {
    pub ports: Ports,
    vs: usize,
    diode: JunctionDiode,
    base_c: f64,
    fwdrop: f64,
    capvoltdiff: f64,
    cap_current: f64,
    comp_r: f64,
    vs_value: f64,
}

impl VaractorElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        capvoltdiff: f64,
        base_c: f64,
        fwdrop: f64,
    ) -> Self {
        let mut ports = Ports::two((x1, y1), (x2, y2), flags);
        ports.alloc_nodes(3);
        Self {
            ports,
            vs: 0,
            diode: JunctionDiode::default_junction(),
            base_c,
            fwdrop,
            capvoltdiff,
            cap_current: 0.0,
            comp_r: 0.0,
            vs_value: 0.0,
        }
    }
}

impl Element for VaractorElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Varactor
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.base_c, "F"))
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (self.node(0), self.node(2))
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
        self.calculate_current();
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ports.current
    }
    fn set_current(&mut self, _vs: usize, c: f64) {
        self.cap_current = c;
    }
    fn reset(&mut self) {
        self.capvoltdiff = 0.0;
        self.cap_current = 0.0;
        self.diode.reset();
        self.ports.current = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        ctx.stamp_voltage_source(self.ports.nodes[0], self.ports.nodes[2], self.vs, None);
    }
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        let c = if self.capvoltdiff > 0.0 {
            self.base_c
        } else {
            self.base_c / (1.0 - self.capvoltdiff / self.fwdrop).powf(0.5)
        };
        self.comp_r = if (self.ports.flags & FLAG_BACK_EULER) == 0 {
            ctx.time_step / (2.0 * c)
        } else {
            ctx.time_step / c
        };
        self.vs_value = -self.capvoltdiff - self.cap_current * self.comp_r;
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let vd = self.ports.volts[0] - self.ports.volts[1];
        self.diode
            .do_step(ctx, self.ports.nodes[0], self.ports.nodes[1], vd);
        ctx.stamp_resistor(self.ports.nodes[2], self.ports.nodes[1], self.comp_r);
        ctx.update_voltage_source(self.vs, self.vs_value);
    }
    fn step_finished(&mut self, _ctx: &mut SimContext) {
        self.capvoltdiff = self.ports.volts[0] - self.ports.volts[1];
    }
    fn calculate_current(&mut self) {
        let vd = self.ports.volts[0] - self.ports.volts[1];
        self.ports.current = self.diode.current(vd) + self.cap_current;
    }
}
