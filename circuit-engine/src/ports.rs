/// Geometry and electrical terminals of an element.
#[derive(Clone, Debug)]
pub struct Ports {
    pub posts: Vec<(i32, i32)>,
    pub nodes: Vec<usize>,
    pub volts: Vec<f64>,
    pub current: f64,
    pub flags: i32,
}

impl Ports {
    pub fn many(posts: Vec<(i32, i32)>, flags: i32) -> Self {
        let n = posts.len();
        Self {
            posts,
            nodes: vec![0; n],
            volts: vec![0.0; n],
            current: 0.0,
            flags,
        }
    }

    pub fn two(p1: (i32, i32), p2: (i32, i32), flags: i32) -> Self {
        Self {
            posts: vec![p1, p2],
            nodes: vec![0, 0],
            volts: vec![0.0, 0.0],
            current: 0.0,
            flags,
        }
    }

    pub fn one(p1: (i32, i32), flags: i32) -> Self {
        Self {
            posts: vec![p1],
            nodes: vec![0],
            volts: vec![0.0],
            current: 0.0,
            flags,
        }
    }

    pub fn alloc_nodes(&mut self, total: usize) {
        self.nodes.resize(total, 0);
        self.volts.resize(total, 0.0);
    }

    pub fn set_node(&mut self, post: usize, node: usize) {
        if post >= self.nodes.len() {
            self.alloc_nodes(post + 1);
        }
        self.nodes[post] = node;
    }

    pub fn set_voltage(&mut self, n: usize, v: f64) {
        if n >= self.volts.len() {
            self.volts.resize(n + 1, 0.0);
        }
        self.volts[n] = v;
    }

    pub fn volt_diff(&self) -> f64 {
        if self.volts.len() < 2 {
            0.0
        } else {
            self.volts[0] - self.volts[1]
        }
    }
}

pub const FLAG_BACK_EULER: i32 = 2;
