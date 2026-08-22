use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::analog_chip_posts;
use crate::ports::Ports;

/// Linear VCVS: V(out+)−V(out−) = gain × (Va−Vb).
pub struct VcvsElm {
    pub ports: Ports,
    pub gain: f64,
    vs: usize,
}

impl VcvsElm {
    pub fn new(x1: i32, y1: i32, x2: i32, _y2: i32, flags: i32, gain: f64) -> Self {
        let _ = x2;
        Self {
            ports: Ports::many(analog_chip_posts((x1, y1), flags), flags),
            gain,
            vs: 0,
        }
    }
}

impl Element for VcvsElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Vcvs
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
        (self.node(3), self.node(2))
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
        let n_out_p = self.ports.nodes[2];
        let n_out_m = self.ports.nodes[3];
        ctx.stamp_voltage_source(n_out_m, n_out_p, self.vs, None);
        ctx.stamp_vs_node(self.vs, self.ports.nodes[0], -self.gain);
        ctx.stamp_vs_node(self.vs, self.ports.nodes[1], self.gain);
    }
}

/// Linear VCCS: I(C+ → C−) = gm × (Va−Vb).
pub struct VccsElm {
    pub ports: Ports,
    pub gm: f64,
}

impl VccsElm {
    pub fn new(x1: i32, y1: i32, x2: i32, _y2: i32, flags: i32, gm: f64) -> Self {
        let _ = x2;
        Self {
            ports: Ports::many(analog_chip_posts((x1, y1), flags), flags),
            gm,
        }
    }
}

impl Element for VccsElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Vccs
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.gm, "S"))
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        (n1 == 2 && n2 == 3) || (n1 == 3 && n2 == 2)
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
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
        ctx.stamp_vc_current_source(
            self.ports.nodes[2],
            self.ports.nodes[3],
            self.ports.nodes[0],
            self.ports.nodes[1],
            self.gm,
        );
    }
    fn calculate_current(&mut self) {
        let vd = self.ports.volts[0] - self.ports.volts[1];
        self.ports.current = self.gm * vd;
    }
}

/// Linear CCVS: V(out+)−V(out−) = gain × I(A+→A−). Dump `214`.
pub struct CcvsElm {
    pub ports: Ports,
    pub gain: f64,
    vs_sense: usize,
    vs_out: usize,
}

impl CcvsElm {
    pub fn new(x1: i32, y1: i32, x2: i32, _y2: i32, flags: i32, gain: f64) -> Self {
        let _ = x2;
        Self {
            ports: Ports::many(analog_chip_posts((x1, y1), flags), flags),
            gain,
            vs_sense: 0,
            vs_out: 0,
        }
    }
}

impl Element for CcvsElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Ccvs
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.gain, "Ω"))
    }
    fn voltage_source_count(&self) -> usize {
        2
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 / 2 == n2 / 2
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn vs_nodes(&self, local: usize) -> (usize, usize) {
        if local == 0 {
            (self.node(0), self.node(1))
        } else {
            (self.node(3), self.node(2))
        }
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
    }
    fn set_voltage_source(&mut self, n: usize, vs: usize) {
        if n == 0 {
            self.vs_sense = vs;
        } else {
            self.vs_out = vs;
        }
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
    fn set_current(&mut self, vs: usize, c: f64) {
        if vs == self.vs_out {
            self.ports.current = c;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        ctx.stamp_voltage_source(
            self.ports.nodes[0],
            self.ports.nodes[1],
            self.vs_sense,
            Some(0.0),
        );
        ctx.stamp_voltage_source(self.ports.nodes[3], self.ports.nodes[2], self.vs_out, None);
        ctx.stamp_vs_vs(self.vs_out, self.vs_sense, -self.gain);
    }
}

/// Linear CCCS: I(O+→O−) = gain × I(A+→A−). Dump `215`.
pub struct CccsElm {
    pub ports: Ports,
    pub gain: f64,
    vs: usize,
}

impl CccsElm {
    pub fn new(x1: i32, y1: i32, x2: i32, _y2: i32, flags: i32, gain: f64) -> Self {
        let _ = x2;
        Self {
            ports: Ports::many(analog_chip_posts((x1, y1), flags), flags),
            gain,
            vs: 0,
        }
    }
}

impl Element for CccsElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Cccs
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.gain, ""))
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 / 2 == n2 / 2
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (self.node(0), self.node(1))
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
        self.ports.current = self.gain * c;
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        ctx.stamp_voltage_source(self.ports.nodes[0], self.ports.nodes[1], self.vs, Some(0.0));
        ctx.stamp_cccs(self.ports.nodes[3], self.ports.nodes[2], self.vs, self.gain);
    }
}
