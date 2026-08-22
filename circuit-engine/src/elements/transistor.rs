use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{dsign, interp2};
use crate::ports::Ports;

const VT: f64 = 0.025865;
const FLAG_FLIP: i32 = 1;

#[derive(Clone, Debug)]
pub struct TransistorModel {
    pub sat_cur: f64,
    pub inv_roll_off_f: f64,
    pub be_leak_cur: f64,
    pub leak_be_n: f64,
    pub inv_roll_off_r: f64,
    pub bc_leak_cur: f64,
    pub leak_bc_n: f64,
    pub emission_f: f64,
    pub emission_r: f64,
    pub inv_early_f: f64,
    pub inv_early_r: f64,
    pub beta_r: f64,
}

impl Default for TransistorModel {
    fn default() -> Self {
        Self {
            sat_cur: 1e-13,
            inv_roll_off_f: 0.0,
            be_leak_cur: 0.0,
            leak_be_n: 1.5,
            inv_roll_off_r: 0.0,
            bc_leak_cur: 0.0,
            leak_bc_n: 2.0,
            emission_f: 1.0,
            emission_r: 1.0,
            inv_early_f: 0.0,
            inv_early_r: 0.0,
            beta_r: 1.0,
        }
    }
}

/// Spice 3f5 BJT (CircuitJS1 TransistorElm), no junction caps.
pub struct TransistorElm {
    pub ports: Ports,
    pub pnp: i32,
    pub beta: f64,
    pub model: TransistorModel,
    last_vbc: f64,
    last_vbe: f64,
    vcrit: f64,
    local_sub_iters: i32,
    ic: f64,
    ib: f64,
}

impl TransistorElm {
    pub fn npn(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::from_dump(x1, y1, x2, y2, 0, 1, 0.0, 0.0, 100.0)
    }

    pub fn pnp(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::from_dump(x1, y1, x2, y2, 0, -1, 0.0, 0.0, 100.0)
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        pnp: i32,
        last_vbe: f64,
        last_vbc: f64,
        beta: f64,
    ) -> Self {
        let mut ds = dsign((x1, y1), (x2, y2));
        if (flags & FLAG_FLIP) != 0 {
            ds = -ds;
        }
        let hs2 = 16 * ds * pnp;
        let (coll, emit) = interp2((x1, y1), (x2, y2), 1.0, hs2 as f64);
        let model = TransistorModel::default();
        let vcrit = VT * (VT / (std::f64::consts::SQRT_2 * model.sat_cur)).ln();
        Self {
            ports: Ports::many(vec![(x1, y1), coll, emit], flags),
            pnp,
            beta,
            model,
            last_vbe,
            last_vbc,
            vcrit,
            local_sub_iters: 0,
            ic: 0.0,
            ib: 0.0,
        }
    }

    fn limit_step(&self, ctx: &mut SimContext, mut vnew: f64, vold: f64) -> f64 {
        if vnew > self.vcrit && (vnew - vold).abs() > (VT + VT) {
            if vold > 0.0 {
                let arg = 1.0 + (vnew - vold) / VT;
                vnew = if arg > 0.0 {
                    vold + VT * arg.ln()
                } else {
                    self.vcrit
                };
            } else {
                vnew = VT * (vnew / VT).ln();
            }
            ctx.converged = false;
        }
        vnew
    }
}

impl Element for TransistorElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Transistor
    }
    fn tag(&self) -> &'static str {
        if self.pnp < 0 {
            "PNP"
        } else {
            "NPN"
        }
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.beta, "β"))
    }
    fn non_linear(&self) -> bool {
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
        self.ic
    }
    fn stamp(&mut self, _ctx: &mut SimContext) {}
    fn do_step(&mut self, ctx: &mut SimContext) {
        let pnp = self.pnp as f64;
        let mut vbc = pnp * (self.ports.volts[0] - self.ports.volts[1]);
        let mut vbe = pnp * (self.ports.volts[0] - self.ports.volts[2]);
        let not_converged = (vbc - self.last_vbc).abs() > 0.01 || (vbe - self.last_vbe).abs() > 0.01;
        if not_converged {
            ctx.converged = false;
            self.local_sub_iters += 1;
        } else {
            self.local_sub_iters = 0;
        }
        let mut gmin = 1e-12;
        if self.local_sub_iters > 100 {
            gmin = (-9.0 * 10f64.ln() * (1.0 - self.local_sub_iters as f64 / 300.0)).exp();
            if gmin > 0.1 {
                gmin = 0.1;
            }
        }
        vbc = self.limit_step(ctx, vbc, self.last_vbc);
        vbe = self.limit_step(ctx, vbe, self.last_vbe);
        self.last_vbc = vbc;
        self.last_vbe = vbe;

        let m = &self.model;
        let csat = m.sat_cur;
        let oik = m.inv_roll_off_f;
        let c2 = m.be_leak_cur;
        let vte = m.leak_be_n * VT;
        let oikr = m.inv_roll_off_r;
        let c4 = m.bc_leak_cur;
        let vtc = m.leak_bc_n * VT;
        let mut vtn = VT * m.emission_f;
        let (cbe, gbe, cben, gben) = if vbe > -5.0 * vtn {
            let evbe = (vbe / vtn).exp();
            let cbe = csat * (evbe - 1.0) + gmin * vbe;
            let gbe = csat * evbe / vtn + gmin;
            if c2 == 0.0 {
                (cbe, gbe, 0.0, 0.0)
            } else {
                let evben = (vbe / vte).exp();
                (cbe, gbe, c2 * (evben - 1.0), c2 * evben / vte)
            }
        } else {
            let gbe = -csat / vbe + gmin;
            let gben = -c2 / vbe;
            (gbe * vbe, gbe, gben * vbe, gben)
        };
        vtn = VT * m.emission_r;
        let (cbc, gbc, cbcn, gbcn) = if vbc > -5.0 * vtn {
            let evbc = (vbc / vtn).exp();
            let cbc = csat * (evbc - 1.0) + gmin * vbc;
            let gbc = csat * evbc / vtn + gmin;
            if c4 == 0.0 {
                (cbc, gbc, 0.0, 0.0)
            } else {
                let evbcn = (vbc / vtc).exp();
                (cbc, gbc, c4 * (evbcn - 1.0), c4 * evbcn / vtc)
            }
        } else {
            let gbc = -csat / vbc + gmin;
            let gbcn = -c4 / vbc;
            (gbc * vbc, gbc, gbcn * vbc, gbcn)
        };
        let q1 = 1.0 / (1.0 - m.inv_early_f * vbc - m.inv_early_r * vbe);
        let (qb, dqbdve, dqbdvc) = if oik == 0.0 && oikr == 0.0 {
            (q1, q1 * q1 * m.inv_early_r, q1 * q1 * m.inv_early_f)
        } else {
            let q2 = oik * cbe + oikr * cbc;
            let arg = (1.0 + 4.0 * q2).max(0.0);
            let sqarg = if arg != 0.0 { arg.sqrt() } else { 1.0 };
            let qb = q1 * (1.0 + sqarg) / 2.0;
            (
                qb,
                q1 * (qb * m.inv_early_r + oik * gbe / sqarg),
                q1 * (qb * m.inv_early_f + oikr * gbc / sqarg),
            )
        };
        let cex = cbe;
        let gex = gbe;
        let cc = (cex - cbc) / qb - cbc / m.beta_r - cbcn;
        let cb = cbe / self.beta + cben + cbc / m.beta_r + cbcn;
        self.ic = pnp * cc;
        self.ib = pnp * cb;
        let gpi = gbe / self.beta + gben;
        let gmu = gbc / m.beta_r + gbcn;
        let go = (gbc + (cex - cbc) * dqbdvc / qb) / qb;
        let gm = (gex - (cex - cbc) * dqbdve / qb) / qb - go;
        let ceqbe = pnp * (cc + cb - vbe * (gm + go + gpi) + vbc * go);
        let ceqbc = pnp * (-cc + vbe * (gm + go) - vbc * (gmu + go));
        let n = &self.ports.nodes;
        ctx.stamp_matrix(n[1], n[1], gmu + go);
        ctx.stamp_matrix(n[1], n[0], -gmu + gm);
        ctx.stamp_matrix(n[1], n[2], -gm - go);
        ctx.stamp_matrix(n[0], n[0], gpi + gmu);
        ctx.stamp_matrix(n[0], n[2], -gpi);
        ctx.stamp_matrix(n[0], n[1], -gmu);
        ctx.stamp_matrix(n[2], n[0], -gpi - gm);
        ctx.stamp_matrix(n[2], n[1], -go);
        ctx.stamp_matrix(n[2], n[2], gpi + gm + go);
        ctx.stamp_right_side(n[0], -ceqbe - ceqbc);
        ctx.stamp_right_side(n[1], ceqbc);
        ctx.stamp_right_side(n[2], ceqbe);
        let _ = self.ib;
    }
    fn reset(&mut self) {
        self.last_vbe = 0.0;
        self.last_vbc = 0.0;
        self.ic = 0.0;
        self.ib = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
}
