use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{tapped_transformer_posts, transformer_posts};
use crate::ports::{Ports, FLAG_BACK_EULER};

/// Linear 4-terminal transformer (CircuitJS1 `TransformerElm`, dump `'T'`).
pub struct TransformerElm {
    pub ports: Ports,
    pub inductance: f64,
    pub ratio: f64,
    pub coupling: f64,
    pub saturation_current: f64,
    current: [f64; 2],
    a: [f64; 4],
    cur_source: [f64; 2],
}

impl TransformerElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        inductance: f64,
        ratio: f64,
        i0: f64,
        i1: f64,
        coupling: f64,
        saturation_current: f64,
    ) -> Self {
        Self {
            ports: Ports::many(transformer_posts((x1, y1), (x2, y2), flags), flags),
            inductance,
            ratio,
            coupling,
            saturation_current,
            current: [i0, i1],
            a: [0.0; 4],
            cur_source: [0.0; 2],
        }
    }

    fn is_trapezoidal(&self) -> bool {
        (self.ports.flags & FLAG_BACK_EULER) == 0
    }

    fn compute_coefficients(&mut self, l1: f64, l2: f64, m: f64, dt: f64) {
        let deti = 1.0 / (l1 * l2 - m * m);
        let ts = if self.is_trapezoidal() { dt / 2.0 } else { dt };
        self.a[0] = l2 * deti * ts;
        self.a[1] = -m * deti * ts;
        self.a[2] = -m * deti * ts;
        self.a[3] = l1 * deti * ts;
    }

    fn effective_l(l0: f64, i: f64, isat: f64) -> f64 {
        if isat <= 0.0 {
            l0
        } else {
            let r = i / isat;
            l0 / (1.0 + r * r)
        }
    }

    fn stamp_g(&self, ctx: &mut SimContext) {
        let n = &self.ports.nodes;
        ctx.stamp_conductance(n[0], n[2], self.a[0]);
        ctx.stamp_vc_current_source(n[0], n[2], n[1], n[3], self.a[1]);
        ctx.stamp_vc_current_source(n[1], n[3], n[0], n[2], self.a[2]);
        ctx.stamp_conductance(n[1], n[3], self.a[3]);
    }
}

impl Element for TransformerElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Transformer
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.inductance, "H"))
    }
    fn non_linear(&self) -> bool {
        self.saturation_current > 0.0
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        matches!((n1, n2), (0, 2) | (2, 0) | (1, 3) | (3, 1))
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
        self.current[0]
    }
    fn reset(&mut self) {
        self.current = [0.0; 2];
        self.cur_source = [0.0; 2];
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        let l1 = self.inductance;
        let l2 = self.inductance * self.ratio * self.ratio;
        let m = self.coupling * (l1 * l2).sqrt();
        self.compute_coefficients(l1, l2, m, ctx.time_step);
        if self.saturation_current <= 0.0 {
            self.stamp_g(ctx);
        }
    }
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        if self.saturation_current > 0.0 {
            let l1 = Self::effective_l(self.inductance, self.current[0], self.saturation_current);
            let l2 = Self::effective_l(
                self.inductance * self.ratio * self.ratio,
                self.current[1],
                self.saturation_current * self.ratio,
            );
            let m = self.coupling * (l1 * l2).sqrt();
            self.compute_coefficients(l1, l2, m, ctx.time_step);
        }
        let v1 = self.ports.volts[0] - self.ports.volts[2];
        let v2 = self.ports.volts[1] - self.ports.volts[3];
        if self.is_trapezoidal() {
            self.cur_source[0] = v1 * self.a[0] + v2 * self.a[1] + self.current[0];
            self.cur_source[1] = v1 * self.a[2] + v2 * self.a[3] + self.current[1];
        } else {
            self.cur_source = self.current;
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        if self.saturation_current > 0.0 {
            self.stamp_g(ctx);
        }
        let n = &self.ports.nodes;
        ctx.stamp_current_source(n[0], n[2], self.cur_source[0]);
        ctx.stamp_current_source(n[1], n[3], self.cur_source[1]);
    }
    fn calculate_current(&mut self) {
        if self.a[0] == 0.0 {
            return;
        }
        let v1 = self.ports.volts[0] - self.ports.volts[2];
        let v2 = self.ports.volts[1] - self.ports.volts[3];
        self.current[0] = v1 * self.a[0] + v2 * self.a[1] + self.cur_source[0];
        self.current[1] = v1 * self.a[2] + v2 * self.a[3] + self.cur_source[1];
        self.ports.current = self.current[0];
    }
}

/// 5-terminal tapped transformer (dump `169`).
pub struct TappedTransformerElm {
    pub ports: Ports,
    pub inductance: f64,
    pub ratio: f64,
    pub coupling: f64,
    current: [f64; 3],
    a: [f64; 9],
    cur_source: [f64; 3],
}

impl TappedTransformerElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        inductance: f64,
        ratio: f64,
        coupling: f64,
    ) -> Self {
        Self {
            ports: Ports::many(tapped_transformer_posts((x1, y1), (x2, y2), flags), flags),
            inductance,
            ratio,
            coupling,
            current: [0.0; 3],
            a: [0.0; 9],
            cur_source: [0.0; 3],
        }
    }

    fn is_trapezoidal(&self) -> bool {
        (self.ports.flags & FLAG_BACK_EULER) == 0
    }
}

impl Element for TappedTransformerElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::TappedTransformer
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.inductance, "H"))
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        matches!(
            (n1.min(n2), n1.max(n2)),
            (0, 1) | (2, 3) | (3, 4) | (2, 4)
        )
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
        self.current[0]
    }
    fn reset(&mut self) {
        self.current = [0.0; 3];
        self.cur_source = [0.0; 3];
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        let l1 = self.inductance;
        let l2 = self.inductance * self.ratio * self.ratio / 4.0;
        let m1 = self.coupling * (l1 * l2).sqrt();
        let m2 = self.coupling * l2;
        self.a[0] = l2 + m2;
        self.a[1] = -m1;
        self.a[2] = -m1;
        self.a[3] = -m1;
        self.a[4] = (l1 * l2 - m1 * m1) / (l2 - m2);
        self.a[5] = (m1 * m1 - l1 * m2) / (l2 - m2);
        self.a[6] = -m1;
        self.a[7] = (m1 * m1 - l1 * m2) / (l2 - m2);
        self.a[8] = (l1 * l2 - m1 * m1) / (l2 - m2);
        let det = l1 * (l2 + m2) - 2.0 * m1 * m1;
        let ts = if self.is_trapezoidal() {
            ctx.time_step / 2.0
        } else {
            ctx.time_step
        };
        for a in &mut self.a {
            *a *= ts / det;
        }
        let n = &self.ports.nodes;
        ctx.stamp_conductance(n[0], n[1], self.a[0]);
        ctx.stamp_vc_current_source(n[0], n[1], n[2], n[3], self.a[1]);
        ctx.stamp_vc_current_source(n[0], n[1], n[3], n[4], self.a[2]);
        ctx.stamp_vc_current_source(n[2], n[3], n[0], n[1], self.a[3]);
        ctx.stamp_conductance(n[2], n[3], self.a[4]);
        ctx.stamp_vc_current_source(n[2], n[3], n[3], n[4], self.a[5]);
        ctx.stamp_vc_current_source(n[3], n[4], n[0], n[1], self.a[6]);
        ctx.stamp_vc_current_source(n[3], n[4], n[2], n[3], self.a[7]);
        ctx.stamp_conductance(n[3], n[4], self.a[8]);
    }
    fn start_iteration(&mut self, _ctx: &mut SimContext) {
        let vd = [
            self.ports.volts[0] - self.ports.volts[1],
            self.ports.volts[2] - self.ports.volts[3],
            self.ports.volts[3] - self.ports.volts[4],
        ];
        for i in 0..3 {
            self.cur_source[i] = self.current[i];
            if self.is_trapezoidal() {
                for j in 0..3 {
                    self.cur_source[i] += self.a[i * 3 + j] * vd[j];
                }
            }
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let n = &self.ports.nodes;
        ctx.stamp_current_source(n[0], n[1], self.cur_source[0]);
        ctx.stamp_current_source(n[2], n[3], self.cur_source[1]);
        ctx.stamp_current_source(n[3], n[4], self.cur_source[2]);
    }
    fn calculate_current(&mut self) {
        if self.a[0] == 0.0 {
            return;
        }
        let vd = [
            self.ports.volts[0] - self.ports.volts[1],
            self.ports.volts[2] - self.ports.volts[3],
            self.ports.volts[3] - self.ports.volts[4],
        ];
        for i in 0..3 {
            self.current[i] = self.cur_source[i];
            for j in 0..3 {
                self.current[i] += self.a[i * 3 + j] * vd[j];
            }
        }
        self.ports.current = self.current[0];
    }
}
