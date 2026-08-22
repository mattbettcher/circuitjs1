use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{dsign, interp2};
use crate::ports::Ports;

const FLAG_PNP: i32 = 1;
const FLAG_FLIP: i32 = 8;

pub struct MosfetElm {
    pub ports: Ports,
    pub pnp: i32,
    pub vt: f64,
    pub beta: f64,
    pub lambda: f64,
    last_v0: f64,
    last_v1: f64,
    last_v2: f64,
    ids: f64,
}

impl MosfetElm {
    pub fn nmos(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::from_dump(x1, y1, x2, y2, 0, 1.5, 0.02)
    }

    pub fn pmos(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::from_dump(x1, y1, x2, y2, FLAG_PNP, 1.5, 0.02)
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        vt: f64,
        beta: f64,
    ) -> Self {
        let pnp = if (flags & FLAG_PNP) != 0 { -1 } else { 1 };
        let mut hs2 = 16 * dsign((x1, y1), (x2, y2));
        if (flags & FLAG_FLIP) != 0 {
            hs2 = -hs2;
        }
        let (src, drn) = interp2((x1, y1), (x2, y2), 1.0, -hs2 as f64);
        Self {
            ports: Ports::many(vec![(x1, y1), src, drn], flags),
            pnp,
            vt,
            beta,
            lambda: 0.0,
            last_v0: 0.0,
            last_v1: 0.0,
            last_v2: 0.0,
            ids: 0.0,
        }
    }

    fn non_convergence(&self, ctx: &SimContext, last: f64, now: f64) -> bool {
        let mut diff = (last - now).abs();
        if self.beta > 1.0 {
            diff *= 100.0;
        }
        if diff < 0.01 {
            return false;
        }
        if ctx.sub_iterations > 10 && diff < now.abs() * 0.001 {
            return false;
        }
        if ctx.sub_iterations > 100 && diff < 0.01 + (ctx.sub_iterations as f64 - 100.0) * 0.0001 {
            return false;
        }
        true
    }
}

impl Element for MosfetElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Mosfet
    }
    fn tag(&self) -> &'static str {
        if self.pnp < 0 {
            "PMOS"
        } else {
            "NMOS"
        }
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        !(n1 == 0 || n2 == 0)
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
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ids
    }
    fn stamp(&mut self, _ctx: &mut SimContext) {}
    fn do_step(&mut self, ctx: &mut SimContext) {
        let mut vs = [
            self.ports.volts[0],
            self.ports.volts[1],
            self.ports.volts[2],
        ];
        vs[1] = vs[1].clamp(self.last_v1 - 0.5, self.last_v1 + 0.5);
        vs[2] = vs[2].clamp(self.last_v2 - 0.5, self.last_v2 + 0.5);
        let mut source = 1usize;
        let mut drain = 2usize;
        if (self.pnp as f64) * vs[1] > (self.pnp as f64) * vs[2] {
            source = 2;
            drain = 1;
        }
        let vgs = vs[0] - vs[source];
        let vds = vs[drain] - vs[source];
        if self.non_convergence(ctx, self.last_v1, vs[1])
            || self.non_convergence(ctx, self.last_v2, vs[2])
            || self.non_convergence(ctx, self.last_v0, vs[0])
        {
            ctx.converged = false;
        }
        self.last_v0 = vs[0];
        self.last_v1 = vs[1];
        self.last_v2 = vs[2];
        let realvgs = vgs;
        let realvds = vds;
        let vgs_p = vgs * self.pnp as f64;
        let vds_p = vds * self.pnp as f64;
        let mut ids;
        let mut gm = 0.0;
        let mut gds;
        if vgs_p < self.vt {
            gds = 1e-8;
            ids = vds_p * gds;
        } else if vds_p < vgs_p - self.vt {
            let lam = self.lambda;
            ids = self.beta
                * ((vgs_p - self.vt) * vds_p - vds_p * vds_p * 0.5)
                * (1.0 + lam * vds_p);
            gm = self.beta * vds_p * (1.0 + lam * vds_p);
            gds = self.beta
                * ((vgs_p - vds_p - self.vt) * (1.0 + lam * vds_p)
                    + lam * ((vgs_p - self.vt) * vds_p - vds_p * vds_p * 0.5));
        } else {
            let lam = self.lambda;
            let vgs_vt = vgs_p - self.vt;
            gm = self.beta * vgs_vt * (1.0 + lam * vds_p);
            gds = 0.5 * self.beta * vgs_vt * vgs_vt * lam;
            if gds < 1e-8 {
                gds = 1e-8;
            }
            ids = 0.5 * self.beta * vgs_vt * vgs_vt * (1.0 + lam * vds_p);
        }
        let ids0 = ids;
        if (source == 2 && self.pnp == 1) || (source == 1 && self.pnp == -1) {
            ids = -ids;
        }
        self.ids = ids;
        let pnp = self.pnp as f64;
        let rs = -pnp * ids0 + gds * realvds + gm * realvgs;
        let n = &self.ports.nodes;
        ctx.stamp_matrix(n[drain], n[drain], gds);
        ctx.stamp_matrix(n[drain], n[source], -gds - gm);
        ctx.stamp_matrix(n[drain], n[0], gm);
        ctx.stamp_matrix(n[source], n[drain], -gds);
        ctx.stamp_matrix(n[source], n[source], gds + gm);
        ctx.stamp_matrix(n[source], n[0], -gm);
        ctx.stamp_right_side(n[drain], rs);
        ctx.stamp_right_side(n[source], -rs);
    }
    fn reset(&mut self) {
        self.last_v0 = 0.0;
        self.last_v1 = 0.0;
        self.last_v2 = 0.0;
        self.ids = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
}
