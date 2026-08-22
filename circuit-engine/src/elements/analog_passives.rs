use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::transformer_posts;
use crate::ports::Ports;

/// Fuse: I²t heating, dump `404`.
pub struct FuseElm {
    pub ports: Ports,
    pub resistance: f64,
    pub i2t: f64,
    heat: f64,
    blown: bool,
}

impl FuseElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        resistance: f64,
        i2t: f64,
        heat: f64,
        blown: bool,
    ) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            resistance,
            i2t,
            heat,
            blown,
        }
    }

    fn r(&self) -> f64 {
        if self.blown {
            1e9
        } else {
            self.resistance
        }
    }
}

impl Element for FuseElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Fuse
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.resistance, "Ω"))
    }
    fn tag(&self) -> &'static str {
        if self.blown {
            "blown"
        } else {
            ""
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
        self.heat = 0.0;
        self.blown = false;
        self.ports.current = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, _ctx: &mut SimContext) {}
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        let i = self.ports.current;
        self.heat += i * i * ctx.time_step;
        self.heat -= ctx.time_step * self.i2t / 3.0;
        if self.heat < 0.0 {
            self.heat = 0.0;
        }
        if self.heat > self.i2t {
            self.blown = true;
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], self.r());
    }
    fn calculate_current(&mut self) {
        self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / self.r();
    }
}

/// Spark gap, dump `187`.
pub struct SparkGapElm {
    pub ports: Ports,
    pub on_r: f64,
    pub off_r: f64,
    pub breakdown: f64,
    pub hold_current: f64,
    state: bool,
}

impl SparkGapElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        on_r: f64,
        off_r: f64,
        breakdown: f64,
        hold_current: f64,
    ) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            on_r,
            off_r,
            breakdown,
            hold_current,
            state: false,
        }
    }

    fn r(&self) -> f64 {
        if self.state {
            self.on_r
        } else {
            self.off_r
        }
    }
}

impl Element for SparkGapElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::SparkGap
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
        self.state = false;
        self.ports.current = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, _ctx: &mut SimContext) {}
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        if self.ports.current.abs() < self.hold_current {
            if self.state {
                ctx.converged = false;
            }
            self.state = false;
        }
        if (self.ports.volts[0] - self.ports.volts[1]).abs() > self.breakdown {
            if !self.state {
                ctx.converged = false;
            }
            self.state = true;
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], self.r());
    }
    fn calculate_current(&mut self) {
        self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / self.r();
    }
}

/// Memristor, dump `'m'`.
pub struct MemristorElm {
    pub ports: Ports,
    pub r_on: f64,
    pub r_off: f64,
    dope_width: f64,
    total_width: f64,
    mobility: f64,
    resistance: f64,
}

impl MemristorElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        r_on: f64,
        r_off: f64,
        dope_width: f64,
        total_width: f64,
        mobility: f64,
        current: f64,
    ) -> Self {
        let mut m = Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            r_on,
            r_off,
            dope_width,
            total_width,
            mobility,
            resistance: r_on,
        };
        m.ports.current = current;
        m
    }
}

impl Element for MemristorElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Memristor
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.resistance, "Ω"))
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
        self.dope_width = 0.0;
        self.ports.current = 0.0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, _ctx: &mut SimContext) {}
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        let wd = self.dope_width / self.total_width;
        self.dope_width +=
            ctx.time_step * self.mobility * self.r_on * self.ports.current / self.total_width;
        self.dope_width = self.dope_width.clamp(0.0, self.total_width);
        self.resistance = self.r_on * wd + self.r_off * (1.0 - wd);
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], self.resistance);
    }
    fn calculate_current(&mut self) {
        self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / self.resistance.max(1e-12);
    }
}

/// Photoresistor, dump `374`. R = (maxLux − lux + 1) × 10.
pub struct LdrElm {
    pub ports: Ports,
    position: f64,
    resistance: f64,
}

impl LdrElm {
    pub fn from_dump(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, position: f64) -> Self {
        let mut e = Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            position: position.clamp(0.0001, 0.9999),
            resistance: 0.0,
        };
        e.update_r();
        e
    }

    fn update_r(&mut self) {
        let min_lux = 0.1;
        let max_lux = 10000.0;
        let lux = max_lux * self.position + min_lux;
        self.resistance = ((max_lux - lux + 1.0) * 10.0).round();
    }
}

impl Element for LdrElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Ldr
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.resistance, "Ω"))
    }
    fn set_slider(&mut self, t: f64) {
        self.position = t.clamp(0.0001, 0.9999);
        self.update_r();
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
        ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], self.resistance);
    }
    fn calculate_current(&mut self) {
        self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / self.resistance;
    }
}

/// NTC thermistor, dump `350`.
pub struct ThermistorElm {
    pub ports: Ports,
    r25: f64,
    r50: f64,
    min_t: f64,
    max_t: f64,
    position: f64,
    resistance: f64,
}

impl ThermistorElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        r25: f64,
        r50: f64,
        min_t: f64,
        max_t: f64,
        position: f64,
    ) -> Self {
        let mut e = Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            r25,
            r50,
            min_t,
            max_t,
            position: position.clamp(0.0, 1.0),
            resistance: r25,
        };
        e.update_r();
        e
    }

    fn update_r(&mut self) {
        let t0 = 273.15;
        let kelvin1 = t0 + 25.0;
        let kelvin2 = t0 + 50.0;
        let b = (self.r25.ln() - self.r50.ln()) / (1.0 / kelvin1 - 1.0 / kelvin2);
        let tempr = (self.position * (self.max_t - self.min_t) + self.min_t).round();
        self.resistance = (self.r25 * (b * (1.0 / (tempr + t0) - 1.0 / (t0 + 25.0))).exp()).round();
    }
}

impl Element for ThermistorElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Thermistor
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.resistance, "Ω"))
    }
    fn set_slider(&mut self, t: f64) {
        self.position = t.clamp(0.0, 1.0);
        self.update_r();
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
        ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], self.resistance);
    }
    fn calculate_current(&mut self) {
        self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / self.resistance;
    }
}

/// Gyrator: I1 = G V2, I2 = −G V1. No text dump type in Java (XML only).
pub struct GyratorElm {
    pub ports: Ports,
    pub resistance: f64,
}

impl GyratorElm {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, resistance: f64) -> Self {
        Self {
            ports: Ports::many(transformer_posts((x1, y1), (x2, y2), flags), flags),
            resistance,
        }
    }
}

impl Element for GyratorElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Gyrator
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.resistance, "Ω"))
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
        self.ports.current
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        let g = 1.0 / self.resistance;
        let n = &self.ports.nodes;
        ctx.stamp_vc_current_source(n[0], n[2], n[1], n[3], g);
        ctx.stamp_vc_current_source(n[1], n[3], n[0], n[2], -g);
    }
    fn calculate_current(&mut self) {
        let g = 1.0 / self.resistance;
        let v1 = self.ports.volts[0] - self.ports.volts[2];
        let v2 = self.ports.volts[1] - self.ports.volts[3];
        self.ports.current = g * v2;
        let _ = v1;
    }
}
