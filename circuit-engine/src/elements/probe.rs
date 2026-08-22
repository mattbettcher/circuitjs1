use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::Ports;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeStyle {
    Output,
    Probe,
    TestPoint,
    Ammeter,
}

/// Voltage probe / output / test point / ammeter. Electrical no-ops except ammeter (0 V VS).
pub struct ProbeElm {
    pub ports: Ports,
    pub style: ProbeStyle,
    vs: usize,
}

impl ProbeElm {
    pub fn output(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            style: ProbeStyle::Output,
            vs: 0,
        }
    }

    pub fn probe(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            style: ProbeStyle::Probe,
            vs: 0,
        }
    }

    pub fn test_point(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            style: ProbeStyle::TestPoint,
            vs: 0,
        }
    }

    pub fn ammeter(x1: i32, y1: i32, x2: i32, y2: i32, flags: i32) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            style: ProbeStyle::Ammeter,
            vs: 0,
        }
    }
}

impl Element for ProbeElm {
    fn post_count(&self) -> usize {
        match self.style {
            ProbeStyle::Output | ProbeStyle::TestPoint => 1,
            ProbeStyle::Probe | ProbeStyle::Ammeter => 2,
        }
    }
    fn posts(&self) -> &[(i32, i32)] {
        let n = self.post_count();
        &self.ports.posts[..n]
    }
    fn geometry(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        match self.style {
            ProbeStyle::Output => ElementKind::Output,
            ProbeStyle::Probe => ElementKind::Probe,
            ProbeStyle::TestPoint => ElementKind::TestPoint,
            ProbeStyle::Ammeter => ElementKind::Ammeter,
        }
    }
    fn voltage_source_count(&self) -> usize {
        if self.style == ProbeStyle::Ammeter {
            1
        } else {
            0
        }
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes.get(post).copied().unwrap_or(0)
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
        if self.style == ProbeStyle::Ammeter {
            ctx.stamp_voltage_source(self.ports.nodes[0], self.ports.nodes[1], self.vs, Some(0.0));
        }
    }
}
