use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

/// Electron thermal voltage at SPICE's default 27 C.
pub const VT: f64 = 0.025865;
const VZCOEF: f64 = 1.0 / VT;

/// Built-in CircuitJS1 "default" diode: Is=1.714e-7, n=2.
pub const DEFAULT_IS: f64 = 1.7143528192808883e-7;
pub const DEFAULT_N: f64 = 2.0;

#[derive(Clone, Debug)]
pub struct DiodeModel {
    pub name: String,
    pub saturation_current: f64,
    pub series_resistance: f64,
    pub emission_coefficient: f64,
    pub breakdown_voltage: f64,
    pub vscale: f64,
    pub vdcoef: f64,
}

impl DiodeModel {
    pub fn new(
        name: &str,
        saturation_current: f64,
        series_resistance: f64,
        emission_coefficient: f64,
        breakdown_voltage: f64,
    ) -> Self {
        let mut m = Self {
            name: name.to_string(),
            saturation_current,
            series_resistance,
            emission_coefficient,
            breakdown_voltage,
            vscale: 0.0,
            vdcoef: 0.0,
        };
        m.update();
        m
    }

    pub fn default_model() -> Self {
        Self::new("default", DEFAULT_IS, 0.0, DEFAULT_N, 0.0)
    }

    pub fn default_zener() -> Self {
        Self::new("default-zener", DEFAULT_IS, 0.0, DEFAULT_N, 5.6)
    }

    pub fn default_led() -> Self {
        Self::new("default-led", 93.2e-12, 0.042, 3.73, 0.0)
    }

    pub fn from_fwdrop(fwdrop: f64, zvoltage: f64) -> Self {
        let emcoef = 2.0;
        let vscale = emcoef * VT;
        let vdcoef = 1.0 / vscale;
        let leakage = 1.0 / ((fwdrop * vdcoef).exp() - 1.0);
        let mut name = format!("fwdrop={fwdrop}");
        if zvoltage != 0.0 {
            name = format!("{name} zvoltage={zvoltage}");
        }
        Self::new(&name, leakage, 0.0, emcoef, zvoltage)
    }

    fn update(&mut self) {
        self.vscale = self.emission_coefficient * VT;
        self.vdcoef = 1.0 / self.vscale;
    }
}

pub const FLAG_FWDROP: i32 = 1;
pub const FLAG_MODEL: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiodeStyle {
    Junction,
    Zener,
    Led,
}

pub struct Diode {
    pub ports: Ports,
    pub model: DiodeModel,
    pub style: DiodeStyle,
    pub diode_end_node: usize,
    leakage: f64,
    zvoltage: f64,
    vscale: f64,
    vdcoef: f64,
    zoffset: f64,
    vcrit: f64,
    vzcrit: f64,
    lastvoltdiff: f64,
}

impl Diode {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::with_style(x1, y1, x2, y2, 0, DiodeModel::default_model(), DiodeStyle::Junction)
    }

    pub fn zener(x1: i32, y1: i32, x2: i32, y2: i32, zvoltage: f64) -> Self {
        Self::with_style(
            x1,
            y1,
            x2,
            y2,
            0,
            DiodeModel::from_fwdrop(0.805904783, zvoltage),
            DiodeStyle::Zener,
        )
    }

    pub fn with_model(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, model: DiodeModel) -> Self {
        Self::with_style(x1, y1, x2, y2, flags, model, DiodeStyle::Junction)
    }

    pub fn with_style(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        model: DiodeModel,
        style: DiodeStyle,
    ) -> Self {
        let mut d = Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            diode_end_node: 1,
            leakage: 0.0,
            zvoltage: 0.0,
            vscale: 0.0,
            vdcoef: 0.0,
            zoffset: 0.0,
            vcrit: 0.0,
            vzcrit: 0.0,
            lastvoltdiff: 0.0,
            model,
            style,
        };
        d.setup();
        d
    }

    fn setup(&mut self) {
        self.leakage = self.model.saturation_current;
        self.zvoltage = self.model.breakdown_voltage;
        self.vscale = self.model.vscale;
        self.vdcoef = self.model.vdcoef;
        self.vcrit = self.vscale * (self.vscale / (std::f64::consts::SQRT_2 * self.leakage)).ln();
        self.vzcrit = VT * (VT / (std::f64::consts::SQRT_2 * self.leakage)).ln();
        if self.zvoltage == 0.0 {
            self.zoffset = 0.0;
        } else {
            let i = -0.005;
            self.zoffset = self.zvoltage - (-(1.0 + i / self.leakage)).ln() / VZCOEF;
        }
        if self.model.series_resistance > 0.0 {
            self.ports.alloc_nodes(3);
            self.diode_end_node = 2;
        } else {
            self.diode_end_node = 1;
        }
    }

    fn limit_step(&mut self, ctx: &mut SimContext, mut vnew: f64, mut vold: f64) -> f64 {
        if vnew > self.vcrit && (vnew - vold).abs() > (self.vscale + self.vscale) {
            if vold > 0.0 {
                let arg = 1.0 + (vnew - vold) / self.vscale;
                if arg > 0.0 {
                    vnew = vold + self.vscale * arg.ln();
                } else {
                    vnew = self.vcrit;
                }
            } else {
                vnew = self.vscale * (vnew / self.vscale).ln();
            }
            ctx.converged = false;
        } else if vnew < 0.0 && self.zoffset != 0.0 {
            vnew = -vnew - self.zoffset;
            vold = -vold - self.zoffset;
            if vnew > self.vzcrit && (vnew - vold).abs() > (VT + VT) {
                if vold > 0.0 {
                    let arg = 1.0 + (vnew - vold) / VT;
                    if arg > 0.0 {
                        vnew = vold + VT * arg.ln();
                    } else {
                        vnew = self.vzcrit;
                    }
                } else {
                    vnew = VT * (vnew / VT).ln();
                }
                ctx.converged = false;
            }
            vnew = -(vnew + self.zoffset);
        }
        vnew
    }
}

impl Element for Diode {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        match self.style {
            DiodeStyle::Junction => ElementKind::Diode,
            DiodeStyle::Zener => ElementKind::Zener,
            DiodeStyle::Led => ElementKind::Led,
        }
    }
    fn internal_node_count(&self) -> usize {
        if self.model.series_resistance > 0.0 {
            1
        } else {
            0
        }
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
        self.calculate_current();
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ports.current
    }
    fn reset(&mut self) {
        self.lastvoltdiff = 0.0;
        self.ports.current = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        if self.model.series_resistance > 0.0 {
            ctx.stamp_resistor(
                self.ports.nodes[1],
                self.ports.nodes[2],
                self.model.series_resistance,
            );
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let n0 = self.ports.nodes[0];
        let n1 = self.ports.nodes[self.diode_end_node];
        let mut voltdiff = self.ports.volts[0] - self.ports.volts[self.diode_end_node];
        if (voltdiff - self.lastvoltdiff).abs() > 0.01 {
            ctx.converged = false;
        }
        voltdiff = self.limit_step(ctx, voltdiff, self.lastvoltdiff);
        self.lastvoltdiff = voltdiff;

        let mut gmin = self.leakage * 0.01;
        if ctx.sub_iterations > 100 {
            gmin = (-9.0 * 10f64.ln() * (1.0 - ctx.sub_iterations as f64 / 3000.0)).exp();
            if gmin > 0.1 {
                gmin = 0.1;
            }
        }

        if voltdiff >= 0.0 || self.zvoltage == 0.0 {
            let eval = (voltdiff * self.vdcoef).exp();
            let geq = self.vdcoef * self.leakage * eval + gmin;
            let nc = (eval - 1.0) * self.leakage - geq * voltdiff;
            ctx.stamp_conductance(n0, n1, geq);
            ctx.stamp_current_source(n0, n1, nc);
        } else {
            let geq = self.leakage
                * (self.vdcoef * (voltdiff * self.vdcoef).exp()
                    + VZCOEF * ((-voltdiff - self.zoffset) * VZCOEF).exp())
                + gmin;
            let nc = self.leakage
                * ((voltdiff * self.vdcoef).exp()
                    - ((-voltdiff - self.zoffset) * VZCOEF).exp()
                    - 1.0)
                + geq * (-voltdiff);
            ctx.stamp_conductance(n0, n1, geq);
            ctx.stamp_current_source(n0, n1, nc);
        }
    }
    fn calculate_current(&mut self) {
        let voltdiff = self.ports.volts[0] - self.ports.volts[self.diode_end_node];
        self.ports.current = if voltdiff >= 0.0 || self.zvoltage == 0.0 {
            self.leakage * ((voltdiff * self.vdcoef).exp() - 1.0)
        } else {
            self.leakage
                * ((voltdiff * self.vdcoef).exp()
                    - ((-voltdiff - self.zoffset) * VZCOEF).exp()
                    - 1.0)
        };
    }
}

/// Gate–source (or similar) Shockley diode used inside JFET / varactor.
pub struct JunctionDiode {
    leakage: f64,
    vscale: f64,
    vdcoef: f64,
    vcrit: f64,
    lastvoltdiff: f64,
}

impl JunctionDiode {
    pub fn default_junction() -> Self {
        let m = DiodeModel::default_model();
        let leakage = m.saturation_current;
        let vscale = m.vscale;
        let vdcoef = m.vdcoef;
        let vcrit = vscale * (vscale / (std::f64::consts::SQRT_2 * leakage)).ln();
        Self {
            leakage,
            vscale,
            vdcoef,
            vcrit,
            lastvoltdiff: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.lastvoltdiff = 0.0;
    }

    pub fn current(&self, voltdiff: f64) -> f64 {
        self.leakage * ((voltdiff * self.vdcoef).exp() - 1.0)
    }

    pub fn do_step(&mut self, ctx: &mut SimContext, n_anode: usize, n_cathode: usize, mut vd: f64) {
        if (vd - self.lastvoltdiff).abs() > 0.01 {
            ctx.converged = false;
        }
        vd = self.limit_step(ctx, vd, self.lastvoltdiff);
        self.lastvoltdiff = vd;
        let mut gmin = self.leakage * 0.01;
        if ctx.sub_iterations > 100 {
            gmin = (-9.0 * 10f64.ln() * (1.0 - ctx.sub_iterations as f64 / 3000.0)).exp();
            if gmin > 0.1 {
                gmin = 0.1;
            }
        }
        let eval = (vd * self.vdcoef).exp();
        let geq = self.vdcoef * self.leakage * eval + gmin;
        let nc = (eval - 1.0) * self.leakage - geq * vd;
        ctx.stamp_conductance(n_anode, n_cathode, geq);
        ctx.stamp_current_source(n_anode, n_cathode, nc);
    }

    fn limit_step(&mut self, ctx: &mut SimContext, mut vnew: f64, vold: f64) -> f64 {
        if vnew > self.vcrit && (vnew - vold).abs() > (self.vscale + self.vscale) {
            if vold > 0.0 {
                let arg = 1.0 + (vnew - vold) / self.vscale;
                if arg > 0.0 {
                    vnew = vold + self.vscale * arg.ln();
                } else {
                    vnew = self.vcrit;
                }
            } else {
                vnew = self.vscale * (vnew / self.vscale).ln();
            }
            ctx.converged = false;
        }
        vnew
    }
}
