use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

const PI: f64 = std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalogSourceKind {
    Sweep,
    Am,
    Fm,
    Antenna,
}

/// One-terminal analog sources: sweep `170`, AM `200`, FM `201`, antenna `'A'`.
pub struct AnalogSourceElm {
    pub ports: Ports,
    vs: usize,
    kind: AnalogSourceKind,
    max_v: f64,
    // sweep
    min_f: f64,
    max_f: f64,
    sweep_time: f64,
    frequency: f64,
    freq_time: f64,
    fadd: f64,
    fmul: f64,
    dir: i32,
    saved_dt: f64,
    sweep_v: f64,
    // AM/FM
    carrier: f64,
    signal: f64,
    deviation: f64,
    last_t: f64,
    funcx: f64,
    fmphase: f64,
}

impl AnalogSourceElm {
    pub fn sweep(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        min_f: f64,
        max_f: f64,
        max_v: f64,
        sweep_time: f64,
    ) -> Self {
        let mut e = Self::base(x1, y1, x2, y2, flags, AnalogSourceKind::Sweep, max_v);
        e.min_f = min_f;
        e.max_f = max_f;
        e.sweep_time = sweep_time;
        e.frequency = min_f;
        e
    }

    pub fn am(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        carrier: f64,
        signal: f64,
        max_v: f64,
    ) -> Self {
        let mut e = Self::base(x1, y1, x2, y2, flags, AnalogSourceKind::Am, max_v);
        e.carrier = carrier;
        e.signal = signal;
        e
    }

    pub fn fm(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        carrier: f64,
        signal: f64,
        max_v: f64,
        deviation: f64,
    ) -> Self {
        let mut e = Self::base(x1, y1, x2, y2, flags, AnalogSourceKind::Fm, max_v);
        e.carrier = carrier;
        e.signal = signal;
        e.deviation = deviation;
        e
    }

    pub fn antenna(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32) -> Self {
        Self::base(x1, y1, x2, y2, flags, AnalogSourceKind::Antenna, 5.0)
    }

    fn base(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        kind: AnalogSourceKind,
        max_v: f64,
    ) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            vs: 0,
            kind,
            max_v,
            min_f: 20.0,
            max_f: 4000.0,
            sweep_time: 0.1,
            frequency: 20.0,
            freq_time: 0.0,
            fadd: 0.0,
            fmul: 1.0,
            dir: 1,
            saved_dt: 0.0,
            sweep_v: 0.0,
            carrier: 1000.0,
            signal: 40.0,
            deviation: 200.0,
            last_t: 0.0,
            funcx: 0.0,
            fmphase: 0.0,
        }
    }

    fn set_sweep_params(&mut self, dt: f64) {
        if self.frequency < self.min_f || self.frequency > self.max_f {
            self.frequency = self.min_f;
            self.freq_time = 0.0;
            self.dir = 1;
        }
        if (self.ports.flags & 1) == 0 {
            self.fadd = self.dir as f64 * dt * (self.max_f - self.min_f) / self.sweep_time;
            self.fmul = 1.0;
        } else {
            self.fadd = 0.0;
            self.fmul = (self.max_f / self.min_f).powf(self.dir as f64 * dt / self.sweep_time);
        }
        self.saved_dt = dt;
    }

    fn voltage(&mut self, ctx: &SimContext) -> f64 {
        if ctx.dc_analysis {
            return 0.0;
        }
        match self.kind {
            AnalogSourceKind::Sweep => self.sweep_v,
            AnalogSourceKind::Am => {
                let w = 2.0 * PI * ctx.t;
                ((w * self.signal).sin() + 1.0) / 2.0 * (w * self.carrier).sin() * self.max_v
            }
            AnalogSourceKind::Fm => {
                let dt = ctx.t - self.last_t;
                self.last_t = ctx.t;
                let amp = (2.0 * PI * ctx.t * self.signal).sin();
                self.funcx += dt * (self.carrier + amp * self.deviation);
                (2.0 * PI * self.funcx).sin() * self.max_v
            }
            AnalogSourceKind::Antenna => {
                let t = ctx.t;
                let fm = 3.0 * self.fmphase.sin();
                (2.0 * PI * t * 3000.0).sin() * (1.3 + (2.0 * PI * t * 12.0).sin()) * 3.0
                    + (2.0 * PI * t * 2710.0).sin() * (1.3 + (2.0 * PI * t * 13.0).sin()) * 3.0
                    + (2.0 * PI * t * 2433.0).sin() * (1.3 + (2.0 * PI * t * 14.0).sin()) * 3.0
                    + fm
            }
        }
    }
}

impl Element for AnalogSourceElm {
    fn post_count(&self) -> usize {
        1
    }
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts[..1]
    }
    fn geometry(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        match self.kind {
            AnalogSourceKind::Sweep => ElementKind::Sweep,
            AnalogSourceKind::Am => ElementKind::AmSource,
            AnalogSourceKind::Fm => ElementKind::FmSource,
            AnalogSourceKind::Antenna => ElementKind::Antenna,
        }
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn is_independent_voltage(&self) -> bool {
        true
    }
    fn has_ground_connection(&self, _post: usize) -> bool {
        true
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (0, self.node(0))
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
    fn reset(&mut self) {
        self.frequency = self.min_f;
        self.freq_time = 0.0;
        self.dir = 1;
        self.funcx = 0.0;
        self.last_t = 0.0;
        self.fmphase = 0.0;
        self.sweep_v = 0.0;
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        ctx.stamp_voltage_source(0, self.ports.nodes[0], self.vs, None);
        if self.kind == AnalogSourceKind::Sweep {
            self.set_sweep_params(ctx.time_step);
        }
    }
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        if self.kind == AnalogSourceKind::Sweep && !ctx.dc_analysis {
            if (ctx.time_step - self.saved_dt).abs() > 0.0 {
                self.set_sweep_params(ctx.time_step);
            }
            self.sweep_v = self.freq_time.sin() * self.max_v;
            self.freq_time += self.frequency * 2.0 * PI * ctx.time_step;
            self.frequency = self.frequency * self.fmul + self.fadd;
            if self.frequency >= self.max_f && self.dir == 1 {
                if (self.ports.flags & 2) != 0 {
                    self.fadd = -self.fadd;
                    self.fmul = 1.0 / self.fmul;
                    self.dir = -1;
                } else {
                    self.frequency = self.min_f;
                }
            }
            if self.frequency <= self.min_f && self.dir == -1 {
                self.fadd = -self.fadd;
                self.fmul = 1.0 / self.fmul;
                self.dir = 1;
            }
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let v = self.voltage(ctx);
        ctx.update_voltage_source(self.vs, v);
    }
    fn step_finished(&mut self, ctx: &mut SimContext) {
        if self.kind == AnalogSourceKind::Antenna && !ctx.dc_analysis {
            self.fmphase +=
                2.0 * PI * (2200.0 + (2.0 * PI * ctx.t * 13.0).sin() * 100.0) * ctx.time_step;
        }
    }
}
