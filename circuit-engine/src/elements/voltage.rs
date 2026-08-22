use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

pub const WF_DC: i32 = 0;
pub const WF_AC: i32 = 1;
pub const WF_SQUARE: i32 = 2;
pub const WF_TRIANGLE: i32 = 3;
pub const WF_SAWTOOTH: i32 = 4;
pub const WF_PULSE: i32 = 5;
pub const WF_VAR: i32 = 7;
pub const FLAG_COS: i32 = 2;

const PI: f64 = std::f64::consts::PI;

pub struct VoltageElm {
    pub ports: Ports,
    pub vs: usize,
    pub waveform: i32,
    pub frequency: f64,
    pub max_voltage: f64,
    pub bias: f64,
    pub phase_shift: f64,
    pub duty_cycle: f64,
    pub freq_time_zero: f64,
    pub is_rail: bool,
}

impl VoltageElm {
    pub fn dc(x1: i32, y1: i32, x2: i32, y2: i32, volts: f64) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), 0),
            vs: 0,
            waveform: WF_DC,
            frequency: 40.0,
            max_voltage: volts,
            bias: 0.0,
            phase_shift: 0.0,
            duty_cycle: 0.5,
            freq_time_zero: 0.0,
            is_rail: false,
        }
    }

    pub fn rail(x1: i32, y1: i32, x2: i32, y2: i32, volts: f64) -> Self {
        let mut v = Self::dc(x1, y1, x2, y2, volts);
        v.is_rail = true;
        v
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        waveform: i32,
        frequency: f64,
        max_voltage: f64,
        bias: f64,
        phase_shift: f64,
        duty_cycle: f64,
    ) -> Self {
        let mut phase = phase_shift;
        let mut f = flags;
        if (f & FLAG_COS) != 0 {
            f &= !FLAG_COS;
            phase = PI / 2.0;
        }
        Self {
            ports: Ports::two((x1, y1), (x2, y2), f),
            vs: 0,
            waveform,
            frequency,
            max_voltage,
            bias,
            phase_shift: phase,
            duty_cycle,
            freq_time_zero: 0.0,
            is_rail: false,
        }
    }

    pub fn from_dump_rail(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        waveform: i32,
        frequency: f64,
        max_voltage: f64,
        bias: f64,
        phase_shift: f64,
        duty_cycle: f64,
    ) -> Self {
        let mut v = Self::from_dump(
            x1, y1, x2, y2, flags, waveform, frequency, max_voltage, bias, phase_shift, duty_cycle,
        );
        v.is_rail = true;
        v
    }

    pub fn voltage(&self, ctx: &SimContext) -> f64 {
        if self.waveform != WF_DC && self.waveform != WF_VAR && ctx.dc_analysis {
            return self.bias;
        }
        let w = 2.0 * PI * (ctx.t - self.freq_time_zero) * self.frequency + self.phase_shift;
        match self.waveform {
            WF_DC => self.max_voltage + self.bias,
            WF_VAR => self.frequency,
            WF_AC => w.sin() * self.max_voltage + self.bias,
            WF_SQUARE => {
                let wm = w.rem_euclid(2.0 * PI);
                let duty_phase = 2.0 * PI * self.duty_cycle;
                if wm < duty_phase {
                    self.max_voltage + self.bias
                } else {
                    -self.max_voltage + self.bias
                }
            }
            WF_TRIANGLE => {
                let x = w.rem_euclid(2.0 * PI);
                let tri = if x < PI {
                    x * (2.0 / PI) - 1.0
                } else {
                    1.0 - (x - PI) * (2.0 / PI)
                };
                tri * self.max_voltage + self.bias
            }
            WF_SAWTOOTH => {
                let x = w.rem_euclid(2.0 * PI) / (2.0 * PI);
                (2.0 * x - 1.0) * self.max_voltage + self.bias
            }
            WF_PULSE => {
                let wm = w.rem_euclid(2.0 * PI);
                if wm < 2.0 * PI * self.duty_cycle {
                    self.max_voltage + self.bias
                } else {
                    self.bias
                }
            }
            _ => self.max_voltage + self.bias,
        }
    }
}

impl Element for VoltageElm {
    fn post_count(&self) -> usize {
        if self.is_rail {
            1
        } else {
            2
        }
    }
    fn posts(&self) -> &[(i32, i32)] {
        if self.is_rail {
            &self.ports.posts[..1]
        } else {
            &self.ports.posts
        }
    }
    fn geometry(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        if self.is_rail {
            ElementKind::Rail
        } else {
            ElementKind::Voltage
        }
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.max_voltage, "V"))
    }
    fn tag(&self) -> &'static str {
        match self.waveform {
            WF_DC => "DC",
            WF_AC => "AC",
            WF_SQUARE => "square",
            WF_TRIANGLE => "triangle",
            WF_SAWTOOTH => "sawtooth",
            WF_PULSE => "pulse",
            WF_VAR => "var",
            _ => "",
        }
    }
    fn set_slider(&mut self, t: f64) {
        if self.waveform == WF_VAR {
            let t = t.clamp(0.0, 1.0);
            self.frequency = self.bias + t * (self.max_voltage - self.bias);
        }
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn is_independent_voltage(&self) -> bool {
        true
    }
    fn has_ground_connection(&self, _post: usize) -> bool {
        self.is_rail
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        if self.is_rail {
            (0, self.node(0))
        } else {
            (self.node(0), self.node(1))
        }
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
        let v = if self.waveform == WF_DC {
            Some(self.voltage(ctx))
        } else {
            None
        };
        let n1 = if self.is_rail { 0 } else { self.ports.nodes[0] };
        let n2 = self.ports.nodes[0];
        let n2 = if self.is_rail { n2 } else { self.ports.nodes[1] };
        ctx.stamp_voltage_source(n1, n2, self.vs, v);
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        if self.waveform != WF_DC {
            let v = self.voltage(ctx);
            ctx.update_voltage_source(self.vs, v);
        }
    }
}
