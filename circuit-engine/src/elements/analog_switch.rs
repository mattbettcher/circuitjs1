use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{analog_switch2_posts, analog_switch_posts};
use crate::ports::Ports;

const FLAG_INVERT: i32 = 1;
const FLAG_PULLDOWN: i32 = 2;

/// Voltage-controlled analog switch, dump `159` (SPST) / `160` (SPDT).
pub struct AnalogSwitchElm {
    pub ports: Ports,
    pub r_on: f64,
    pub r_off: f64,
    pub threshold: f64,
    spdt: bool,
    open: bool,
}

impl AnalogSwitchElm {
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        r_on: f64,
        r_off: f64,
        threshold: f64,
        spdt: bool,
    ) -> Self {
        let posts = if spdt {
            analog_switch2_posts((x1, y1), (x2, y2))
        } else {
            analog_switch_posts((x1, y1), (x2, y2))
        };
        Self {
            ports: Ports::many(posts, flags),
            r_on,
            r_off,
            threshold,
            spdt,
            open: false,
        }
    }

    fn needs_pulldown(&self) -> bool {
        (self.ports.flags & FLAG_PULLDOWN) != 0
    }
}

impl Element for AnalogSwitchElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        if self.spdt {
            ElementKind::AnalogSwitchSpdt
        } else {
            ElementKind::AnalogSwitch
        }
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        let ctrl = if self.spdt { 3 } else { 2 };
        n1 != ctrl && n2 != ctrl
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        if !self.needs_pulldown() {
            return false;
        }
        if self.spdt {
            n == 1 || n == 2
        } else {
            n < 2
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
        self.calculate_current();
    }
    fn volts(&self) -> &[f64] {
        &self.ports.volts
    }
    fn current(&self) -> f64 {
        self.ports.current
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        if self.needs_pulldown() {
            if self.spdt {
                ctx.stamp_resistor(self.ports.nodes[1], 0, self.r_off);
                ctx.stamp_resistor(self.ports.nodes[2], 0, self.r_off);
            } else {
                ctx.stamp_resistor(self.ports.nodes[0], 0, self.r_off);
                ctx.stamp_resistor(self.ports.nodes[1], 0, self.r_off);
            }
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let ctrl = if self.spdt { 3 } else { 2 };
        let was_open = self.open;
        self.open = self.ports.volts[ctrl] < self.threshold;
        if (self.ports.flags & FLAG_INVERT) != 0 {
            self.open = !self.open;
        }
        if self.open != was_open {
            ctx.converged = false;
        }
        let n = &self.ports.nodes;
        if self.spdt {
            if self.open {
                ctx.stamp_resistor(n[0], n[2], self.r_on);
                if !self.needs_pulldown() {
                    ctx.stamp_resistor(n[0], n[1], self.r_off);
                }
            } else {
                ctx.stamp_resistor(n[0], n[1], self.r_on);
                if !self.needs_pulldown() {
                    ctx.stamp_resistor(n[0], n[2], self.r_off);
                }
            }
        } else if !(self.needs_pulldown() && self.open) {
            let r = if self.open { self.r_off } else { self.r_on };
            ctx.stamp_resistor(n[0], n[1], r);
        }
    }
    fn calculate_current(&mut self) {
        if self.spdt {
            self.ports.current = if self.open {
                (self.ports.volts[0] - self.ports.volts[2]) / self.r_on
            } else {
                (self.ports.volts[0] - self.ports.volts[1]) / self.r_on
            };
        } else if self.needs_pulldown() && self.open {
            self.ports.current = 0.0;
        } else {
            let r = if self.open { self.r_off } else { self.r_on };
            self.ports.current = (self.ports.volts[0] - self.ports.volts[1]) / r;
        }
    }
}
