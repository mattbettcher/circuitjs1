use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::{Ports, FLAG_BACK_EULER};

/// Quartz crystal as C_parallel ∥ (C_series–L–R), dump `412`.
pub struct CrystalElm {
    pub ports: Ports,
    pub parallel_c: f64,
    pub series_c: f64,
    pub inductance: f64,
    pub resistance: f64,
    voltdiff_p: f64,
    voltdiff_s: f64,
    i_p: f64,
    i_s: f64,
    i_l: f64,
    comp_rp: f64,
    comp_rs: f64,
    comp_rl: f64,
    cur_p: f64,
    cur_s: f64,
    cur_l: f64,
}

impl CrystalElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        parallel_c: f64,
        series_c: f64,
        inductance: f64,
        resistance: f64,
    ) -> Self {
        let mut ports = Ports::two((x1, y1), (x2, y2), flags);
        ports.alloc_nodes(4);
        Self {
            ports,
            parallel_c,
            series_c,
            inductance,
            resistance,
            voltdiff_p: 0.0,
            voltdiff_s: 0.0,
            i_p: 0.0,
            i_s: 0.0,
            i_l: 0.0,
            comp_rp: 0.0,
            comp_rs: 0.0,
            comp_rl: 0.0,
            cur_p: 0.0,
            cur_s: 0.0,
            cur_l: 0.0,
        }
    }

    fn trap(&self) -> bool {
        (self.ports.flags & FLAG_BACK_EULER) == 0
    }
}

impl Element for CrystalElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Crystal
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.inductance, "H"))
    }
    fn internal_node_count(&self) -> usize {
        2
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
    fn reset(&mut self) {
        self.voltdiff_p = 0.0;
        self.voltdiff_s = 0.0;
        self.i_p = 0.0;
        self.i_s = 0.0;
        self.i_l = 0.0;
        self.cur_p = 0.0;
        self.cur_s = 0.0;
        self.cur_l = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], 1e8);
            return;
        }
        let dt = ctx.time_step;
        self.comp_rp = if self.trap() {
            dt / (2.0 * self.parallel_c)
        } else {
            dt / self.parallel_c
        };
        self.comp_rs = if self.trap() {
            dt / (2.0 * self.series_c)
        } else {
            dt / self.series_c
        };
        self.comp_rl = if self.trap() {
            2.0 * self.inductance / dt
        } else {
            self.inductance / dt
        };
        let n0 = self.ports.nodes[0];
        let n1 = self.ports.nodes[1];
        let n2 = self.ports.nodes[2];
        let n3 = self.ports.nodes[3];
        ctx.stamp_resistor(n0, n1, self.comp_rp);
        ctx.stamp_resistor(n0, n2, self.comp_rs);
        ctx.stamp_resistor(n2, n3, self.comp_rl);
        ctx.stamp_resistor(n3, n1, self.resistance);
    }
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            return;
        }
        if self.trap() {
            self.cur_p = -self.voltdiff_p / self.comp_rp - self.i_p;
            self.cur_s = -self.voltdiff_s / self.comp_rs - self.i_s;
            let vl = self.ports.volts[2] - self.ports.volts[3];
            self.cur_l = vl / self.comp_rl + self.i_l;
        } else {
            self.cur_p = -self.voltdiff_p / self.comp_rp;
            self.cur_s = -self.voltdiff_s / self.comp_rs;
            self.cur_l = self.i_l;
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            return;
        }
        let n0 = self.ports.nodes[0];
        let n1 = self.ports.nodes[1];
        let n2 = self.ports.nodes[2];
        let n3 = self.ports.nodes[3];
        ctx.stamp_current_source(n0, n1, self.cur_p);
        ctx.stamp_current_source(n0, n2, self.cur_s);
        ctx.stamp_current_source(n2, n3, self.cur_l);
    }
    fn step_finished(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            return;
        }
        self.voltdiff_p = self.ports.volts[0] - self.ports.volts[1];
        self.voltdiff_s = self.ports.volts[0] - self.ports.volts[2];
        if self.comp_rp > 0.0 {
            self.i_p = self.voltdiff_p / self.comp_rp + self.cur_p;
        }
        if self.comp_rs > 0.0 {
            self.i_s = self.voltdiff_s / self.comp_rs + self.cur_s;
        }
        if self.comp_rl > 0.0 {
            let vl = self.ports.volts[2] - self.ports.volts[3];
            self.i_l = vl / self.comp_rl + self.cur_l;
        }
        self.ports.current = self.i_p + self.i_s;
    }
}
