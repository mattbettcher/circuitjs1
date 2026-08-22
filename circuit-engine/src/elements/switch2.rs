use crate::context::SimContext;
use crate::element::{Element, ElementKind};
use crate::geom::{interp_off, interp2};
use crate::ports::Ports;

const FLAG_CENTER_OFF: i32 = 1;
const OPEN_HS: f64 = 16.0;

/// SPDT (or n-throw) switch. Common is post 0; throws are posts 1..n.
pub struct Switch2Elm {
    pub ports: Ports,
    pub position: i32,
    pub pos_count: i32,
    pub throw_count: i32,
    pub momentary: bool,
    pub resistance: f64,
    pub link: i32,
    vs: usize,
}

impl Switch2Elm {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::from_dump(x1, y1, x2, y2, 0, 0, false, 0, 2)
    }

    pub fn from_dump(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        position: i32,
        momentary: bool,
        link: i32,
        throw_count: i32,
    ) -> Self {
        let n = throw_count.max(2);
        let mut posts = vec![(x1, y1)];
        if n == 2 {
            let (a, b) = interp2((x1, y1), (x2, y2), 1.0, OPEN_HS);
            posts.push(a);
            posts.push(b);
        } else {
            for i in 0..n {
                let hs = -OPEN_HS * (i as f64 - (n - 1) as f64 / 2.0);
                posts.push(interp_off((x1, y1), (x2, y2), 1.0, hs));
            }
        }
        let pos_count = if (flags & FLAG_CENTER_OFF) != 0 && n == 2 {
            3
        } else {
            n
        };
        Self {
            ports: Ports::many(posts, flags),
            position,
            pos_count,
            throw_count: n,
            momentary,
            resistance: 0.0,
            link,
            vs: 0,
        }
    }

    fn has_center_off(&self) -> bool {
        (self.ports.flags & FLAG_CENTER_OFF) != 0 && self.throw_count == 2
    }

    fn closed(&self) -> bool {
        !(self.has_center_off() && self.position == 2)
    }
}

impl Element for Switch2Elm {
    fn posts(&self) -> &[(i32, i32)] {
        &self.ports.posts
    }
    fn kind(&self) -> ElementKind {
        ElementKind::SwitchSpdt
    }
    fn voltage_source_count(&self) -> usize {
        if !self.closed() || self.resistance > 0.0 {
            0
        } else {
            1
        }
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        let t = (self.position + 1) as usize;
        (self.node(0), self.node(t.min(self.post_count() - 1)))
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        if !self.closed() {
            return false;
        }
        let t = (self.position + 1) as usize;
        (n1 == 0 && n2 == t) || (n2 == 0 && n1 == t)
    }
    fn toggle(&mut self) -> bool {
        self.position += 1;
        if self.position >= self.pos_count {
            self.position = 0;
        }
        true
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
        self.calculate_current();
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
        if !self.closed() {
            return;
        }
        let t = (self.position + 1) as usize;
        let n0 = self.ports.nodes[0];
        let nt = self.ports.nodes[t];
        if self.resistance > 0.0 {
            ctx.stamp_resistor(n0, nt, self.resistance);
        } else {
            ctx.stamp_voltage_source(n0, nt, self.vs, Some(0.0));
        }
    }
    fn calculate_current(&mut self) {
        if !self.closed() {
            self.ports.current = 0.0;
        } else if self.resistance > 0.0 {
            let t = (self.position + 1) as usize;
            self.ports.current =
                (self.ports.volts[0] - self.ports.volts[t]) / self.resistance;
        }
    }
}
