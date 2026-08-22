use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::interp_off;
use crate::ports::Ports;

const FLAG_PULLDOWN: i32 = 16;

/// Multi-pole relay with inductive coil (dump `178`). Default model is 1 pole.
pub struct RelayElm {
    pub ports: Ports,
    poles: usize,
    inductance: f64,
    r_on: f64,
    r_off: f64,
    on_current: f64,
    off_current: f64,
    coil_r: f64,
    switching_time: f64,
    coil_current: f64,
    d_position: f64,
    i_position: i32,
    on_state: bool,
    comp_resistance: f64,
    cur_source: f64,
}

impl RelayElm {
    #[allow(clippy::too_many_arguments)]
    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        poles: i32,
        inductance: f64,
        coil_current: f64,
        r_on: f64,
        r_off: f64,
        on_current: f64,
        coil_r: f64,
        off_current: f64,
        switching_time: f64,
        i_position: i32,
    ) -> Self {
        let poles = poles.max(1) as usize;
        let mut posts = Vec::with_capacity(2 + poles * 3);
        for p in 0..poles {
            let yoff = (p as f64) * 24.0;
            posts.push(interp_off((x1, y1), (x2, y2), 0.35, yoff));
            posts.push(interp_off((x1, y1), (x2, y2), 0.0, yoff));
            posts.push(interp_off((x1, y1), (x2, y2), 1.0, yoff));
        }
        posts.push(interp_off((x1, y1), (x2, y2), 0.0, -40.0));
        posts.push(interp_off((x1, y1), (x2, y2), 1.0, -40.0));
        let mut ports = Ports::many(posts, flags);
        ports.alloc_nodes(2 + poles * 3 + 1);
        let mut e = Self {
            ports,
            poles,
            inductance,
            r_on,
            r_off,
            on_current,
            off_current,
            coil_r,
            switching_time,
            coil_current,
            d_position: i_position.clamp(0, 1) as f64,
            i_position,
            on_state: i_position == 1,
            comp_resistance: 0.0,
            cur_source: coil_current,
        };
        if e.i_position == 2 {
            e.d_position = 0.5;
        }
        e
    }

    fn n_coil1(&self) -> usize {
        3 * self.poles
    }
    fn n_coil2(&self) -> usize {
        3 * self.poles + 1
    }
    fn n_coil3(&self) -> usize {
        3 * self.poles + 2
    }
    fn needs_pulldown(&self) -> bool {
        (self.ports.flags & FLAG_PULLDOWN) != 0
    }
}

impl Element for RelayElm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::Relay
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 / 3 == n2 / 3
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        self.needs_pulldown() && n < self.n_coil1()
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
        self.coil_current
    }
    fn reset(&mut self) {
        self.coil_current = 0.0;
        self.cur_source = 0.0;
        self.d_position = 0.0;
        self.i_position = 0;
        for v in &mut self.ports.volts {
            *v = 0.0;
        }
    }
    fn stamp(&mut self, ctx: &mut SimContext) {
        // Java inductor is always backward-Euler for the coil.
        self.comp_resistance = self.inductance / ctx.time_step;
        let n1 = self.ports.nodes[self.n_coil1()];
        let n2 = self.ports.nodes[self.n_coil2()];
        let n3 = self.ports.nodes[self.n_coil3()];
        ctx.stamp_resistor(n1, n3, self.comp_resistance);
        ctx.stamp_resistor(n3, n2, self.coil_r);
        if self.needs_pulldown() {
            for p in 0..self.poles {
                ctx.stamp_resistor(self.ports.nodes[1 + p * 3], 0, self.r_off);
                ctx.stamp_resistor(self.ports.nodes[2 + p * 3], 0, self.r_off);
            }
        }
    }
    fn start_iteration(&mut self, ctx: &mut SimContext) {
        self.cur_source = self.coil_current;

        if self.switching_time == 0.0 {
            let magic = 1.3;
            let pmult = (magic + 1.0_f64).sqrt();
            let p = self.coil_current * pmult / self.on_current;
            self.d_position = (p * p).abs() - 1.3;
            self.d_position = self.d_position.clamp(0.0, 1.0);
            self.i_position = if self.d_position < 0.1 {
                0
            } else if self.d_position > 0.9 {
                1
            } else {
                2
            };
            return;
        }

        let abs_i = self.coil_current.abs();
        if self.on_state {
            if abs_i < self.off_current {
                self.on_state = false;
                self.i_position = 2;
            } else {
                self.d_position += ctx.time_step / self.switching_time;
                if self.d_position >= 1.0 {
                    self.d_position = 1.0;
                    self.i_position = 1;
                }
            }
        } else if abs_i > self.on_current {
            self.on_state = true;
            self.i_position = 2;
        } else {
            self.d_position -= ctx.time_step / self.switching_time;
            if self.d_position <= 0.0 {
                self.d_position = 0.0;
                self.i_position = 0;
            }
        }
    }
    fn do_step(&mut self, ctx: &mut SimContext) {
        let n1 = self.ports.nodes[self.n_coil1()];
        let n3 = self.ports.nodes[self.n_coil3()];
        ctx.stamp_current_source(n1, n3, self.cur_source);
        for p in 0..self.poles {
            let a = self.ports.nodes[p * 3];
            let b = self.ports.nodes[p * 3 + 1];
            let c = self.ports.nodes[p * 3 + 2];
            match self.i_position {
                0 => {
                    ctx.stamp_resistor(a, b, self.r_on);
                    if !self.needs_pulldown() {
                        ctx.stamp_resistor(a, c, self.r_off);
                    }
                }
                1 => {
                    ctx.stamp_resistor(a, c, self.r_on);
                    if !self.needs_pulldown() {
                        ctx.stamp_resistor(a, b, self.r_off);
                    }
                }
                _ => {
                    ctx.stamp_resistor(a, b, self.r_off);
                    ctx.stamp_resistor(a, c, self.r_off);
                }
            }
        }
    }
    fn calculate_current(&mut self) {
        if self.comp_resistance > 0.0 {
            let n1 = self.n_coil1();
            let n3 = self.n_coil3();
            let voltdiff = self.ports.volts[n1] - self.ports.volts[n3];
            self.coil_current = voltdiff / self.comp_resistance + self.cur_source;
            self.ports.current = self.coil_current;
        }
    }
}
