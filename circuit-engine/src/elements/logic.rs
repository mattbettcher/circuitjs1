use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

const FLAG_TERNARY: i32 = 1;
const FLAG_NUMERIC: i32 = 2;
const FLAG_PULLDOWN: i32 = 4;

/// Logic input, dump `L`. One-terminal VS to ground at `loV` / `hiV`.
pub struct LogicInputElm {
    pub ports: Ports,
    pub position: i32,
    pub pos_count: i32,
    pub momentary: bool,
    pub hi_v: f64,
    pub lo_v: f64,
    vs: usize,
}

impl LogicInputElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        position: i32,
        momentary: bool,
        hi_v: f64,
        lo_v: f64,
    ) -> Self {
        let ternary = (flags & FLAG_TERNARY) != 0;
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            position,
            pos_count: if ternary { 3 } else { 2 },
            momentary,
            hi_v,
            lo_v,
            vs: 0,
        }
    }

    fn output_voltage(&self) -> f64 {
        if (self.ports.flags & FLAG_TERNARY) != 0 {
            self.lo_v + self.position as f64 * (self.hi_v - self.lo_v) * 0.5
        } else if self.position == 0 {
            self.lo_v
        } else {
            self.hi_v
        }
    }
}

impl Element for LogicInputElm {
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
        ElementKind::LogicInput
    }
    fn tag(&self) -> &'static str {
        if (self.ports.flags & (FLAG_TERNARY | FLAG_NUMERIC)) != 0 {
            match self.position {
                0 => "0",
                1 => "1",
                _ => "2",
            }
        } else if self.position == 0 {
            "L"
        } else {
            "H"
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
    fn toggle(&mut self) -> bool {
        self.position += 1;
        if self.position >= self.pos_count {
            self.position = 0;
        }
        false
    }
    fn position(&self) -> i32 {
        self.position
    }
    fn set_position(&mut self, p: i32) {
        self.position = p.clamp(0, self.pos_count - 1);
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
        ctx.stamp_voltage_source(0, self.ports.nodes[0], self.vs, None);
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        ctx.update_voltage_source(self.vs, self.output_voltage());
    }
}

/// Logic output, dump `M`. Probe with optional 1 MΩ pulldown.
pub struct LogicOutputElm {
    pub ports: Ports,
    pub threshold: f64,
}

impl LogicOutputElm {
    pub fn from_dump(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32, threshold: f64) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            threshold,
        }
    }
}

impl Element for LogicOutputElm {
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
        ElementKind::LogicOutput
    }
    fn tag(&self) -> &'static str {
        let v = self.ports.volts.first().copied().unwrap_or(0.0);
        let ternary = (self.ports.flags & FLAG_TERNARY) != 0;
        let numeric = (self.ports.flags & (FLAG_TERNARY | FLAG_NUMERIC)) != 0;
        if ternary {
            if v > self.threshold * 1.5 {
                "2"
            } else if v > self.threshold * 0.5 {
                "1"
            } else {
                "0"
            }
        } else if numeric {
            if v < self.threshold {
                "0"
            } else {
                "1"
            }
        } else if v < self.threshold {
            "L"
        } else {
            "H"
        }
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
        0.0
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        if (self.ports.flags & FLAG_PULLDOWN) != 0 {
            ctx.stamp_resistor(self.ports.nodes[0], 0, 1e6);
        }
    }
}

/// Inverter, dump `I`. Output VS to ground with slew limiting.
pub struct InverterElm {
    pub ports: Ports,
    vs: usize,
    slew_rate: f64,
    high_voltage: f64,
    last_out: f64,
}

impl InverterElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        slew_rate: f64,
        high_voltage: f64,
    ) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            vs: 0,
            slew_rate,
            high_voltage,
            last_out: 0.0,
        }
    }
}

impl Element for InverterElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Inverter
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        n == 1
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (0, self.node(1))
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
        ctx.stamp_voltage_source(0, self.ports.nodes[1], self.vs, None);
    }
    fn start_iteration(&mut self, _ctx: &mut SimContext) {
        self.last_out = self.ports.volts[1];
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let mut out = if self.ports.volts[0] > self.high_voltage * 0.5 {
            0.0
        } else {
            self.high_voltage
        };
        let max_step = self.slew_rate * ctx.time_step * 1e9;
        out = out.clamp(self.last_out - max_step, self.last_out + max_step);
        ctx.update_voltage_source(self.vs, out);
    }
}
