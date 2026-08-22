use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::ports::{Ports, FLAG_BACK_EULER};

pub struct Capacitor {
    pub ports: Ports,
    pub capacitance: f64,
    pub voltdiff: f64,
    pub initial_voltage: f64,
    pub series_resistance: f64,
    pub cap_node2: usize,
    pub comp_resistance: f64,
    pub cur_source_value: f64,
    dc_omit_internal: bool,
}

impl Capacitor {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, capacitance: f64) -> Self {
        Self {
            ports: Ports::two((x1, y1), (x2, y2), 0),
            capacitance,
            voltdiff: 0.0,
            initial_voltage: 0.0,
            series_resistance: 0.0,
            cap_node2: 1,
            comp_resistance: 0.0,
            cur_source_value: 0.0,
            dc_omit_internal: false,
        }
    }

    pub fn with_state(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        capacitance: f64,
        voltdiff: f64,
        initial_voltage: f64,
        series_resistance: f64,
    ) -> Self {
        let mut c = Self {
            ports: Ports::two((x1, y1), (x2, y2), flags),
            capacitance,
            voltdiff,
            initial_voltage,
            series_resistance,
            cap_node2: 1,
            comp_resistance: 0.0,
            cur_source_value: 0.0,
            dc_omit_internal: false,
        };
        if series_resistance > 0.0 {
            c.ports.alloc_nodes(3);
        }
        c
    }

    fn is_trapezoidal(&self) -> bool {
        (self.ports.flags & FLAG_BACK_EULER) == 0
    }
}

impl Element for Capacitor {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Capacitor
    }
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        Some((self.capacitance, "F"))
    }
    fn internal_node_count(&self) -> usize {
        if self.series_resistance > 0.0 && !self.dc_omit_internal {
            1
        } else {
            0
        }
    }
    fn prepare(&mut self, dc_analysis: bool) {
        self.dc_omit_internal = dc_analysis;
        if dc_analysis || self.series_resistance <= 0.0 {
            self.ports.alloc_nodes(2);
        } else {
            self.ports.alloc_nodes(3);
        }
    }
    fn set_node(&mut self, post: usize, node: usize) {
        self.ports.set_node(post, node);
    }
    fn node(&self, post: usize) -> usize {
        self.ports.nodes[post]
    }
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        // Do not calculate current here (matches CapacitorElm).
        self.ports.set_voltage(n, v);
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ports.current
    }
    fn reset(&mut self) {
        self.ports.current = 0.0;
        self.cur_source_value = 0.0;
        self.voltdiff = self.initial_voltage;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            ctx.stamp_resistor(self.ports.nodes[0], self.ports.nodes[1], 1e8);
            self.cur_source_value = 0.0;
            self.cap_node2 = 1;
            return;
        }
        self.cap_node2 = if self.series_resistance > 0.0 { 2 } else { 1 };
        self.comp_resistance = if self.is_trapezoidal() {
            ctx.time_step / (2.0 * self.capacitance)
        } else {
            ctx.time_step / self.capacitance
        };
        ctx.stamp_resistor(
            self.ports.nodes[0],
            self.ports.nodes[self.cap_node2],
            self.comp_resistance,
        );
        if self.series_resistance > 0.0 {
            ctx.stamp_resistor(
                self.ports.nodes[1],
                self.ports.nodes[2],
                self.series_resistance,
            );
        }
    }
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            return;
        }
        if self.is_trapezoidal() {
            self.cur_source_value = -self.voltdiff / self.comp_resistance - self.ports.current;
        } else {
            self.cur_source_value = -self.voltdiff / self.comp_resistance;
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        if ctx.dc_analysis {
            return;
        }
        ctx.stamp_current_source(
            self.ports.nodes[0],
            self.ports.nodes[self.cap_node2],
            self.cur_source_value,
        );
    }
    fn step_finished(&mut self, _ctx: &mut SimContext) {
        self.voltdiff = self.ports.volts[0] - self.ports.volts[self.cap_node2];
        self.calculate_current();
    }
    fn calculate_current(&mut self) {
        let voltdiff = self.ports.volts[0] - self.ports.volts[self.cap_node2];
        if self.comp_resistance > 0.0 {
            self.ports.current = voltdiff / self.comp_resistance + self.cur_source_value;
        }
    }
}
