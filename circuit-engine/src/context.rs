use crate::matrix::{CircuitMatrix, Node, VoltageSource};

/// Mutable simulation state seen by elements while stamping.
pub struct SimContext {
    pub matrices: Vec<CircuitMatrix>,
    pub nodes: Vec<Node>,
    pub voltage_sources: Vec<VoltageSource>,
    pub time_step: f64,
    pub t: f64,
    pub dc_analysis: bool,
    pub converged: bool,
    pub sub_iterations: i32,
}

impl SimContext {
    pub fn stamp_resistor(&mut self, n1: usize, n2: usize, r: f64) {
        self.stamp_conductance(n1, n2, 1.0 / r);
    }

    pub fn stamp_conductance(&mut self, n1: usize, n2: usize, g: f64) {
        self.add_a_nn(n1, n1, g);
        self.add_a_nn(n2, n2, g);
        self.add_a_nn(n1, n2, -g);
        self.add_a_nn(n2, n1, -g);
    }

    pub fn stamp_current_source(&mut self, n1: usize, n2: usize, i: f64) {
        self.add_rhs_n(n1, -i);
        self.add_rhs_n(n2, i);
    }

    /// Independent voltage source. `v` is `Some` when the value is known at stamp time (DC).
    pub fn stamp_voltage_source(&mut self, n1: usize, n2: usize, vs: usize, v: Option<f64>) {
        self.add_a_vn(vs, n1, -1.0);
        self.add_a_vn(vs, n2, 1.0);
        self.add_a_nv(n1, vs, 1.0);
        self.add_a_nv(n2, vs, -1.0);
        if let Some(volt) = v {
            self.add_rhs_v(vs, volt);
        }
    }

    pub fn update_voltage_source(&mut self, vs: usize, v: f64) {
        self.add_rhs_v(vs, v);
    }

    fn add_a_nn(&mut self, i: usize, j: usize, x: f64) {
        if i == 0 || j == 0 {
            return;
        }
        let (mi, ri) = match self.node_loc(i) {
            Some(l) => l,
            None => return,
        };
        let (mj, rj) = match self.node_loc(j) {
            Some(l) => l,
            None => return,
        };
        debug_assert_eq!(mi, mj);
        self.matrices[mi].a[ri][rj] += x;
    }

    fn add_a_vn(&mut self, vs: usize, j: usize, x: f64) {
        if j == 0 {
            return;
        }
        let v = &self.voltage_sources[vs];
        let (mj, rj) = match self.node_loc(j) {
            Some(l) => l,
            None => return,
        };
        debug_assert_eq!(v.matrix, mj);
        self.matrices[v.matrix].a[v.row][rj] += x;
    }

    fn add_a_nv(&mut self, i: usize, vs: usize, x: f64) {
        if i == 0 {
            return;
        }
        let v = &self.voltage_sources[vs];
        let (mi, ri) = match self.node_loc(i) {
            Some(l) => l,
            None => return,
        };
        debug_assert_eq!(v.matrix, mi);
        self.matrices[v.matrix].a[ri][v.row] += x;
    }

    fn add_rhs_n(&mut self, n: usize, x: f64) {
        if n == 0 {
            return;
        }
        if let Some((mi, ri)) = self.node_loc(n) {
            self.matrices[mi].rhs[ri] += x;
        }
    }

    fn add_rhs_v(&mut self, vs: usize, x: f64) {
        let v = &self.voltage_sources[vs];
        self.matrices[v.matrix].rhs[v.row] += x;
    }

    fn node_loc(&self, n: usize) -> Option<(usize, usize)> {
        let node = self.nodes.get(n)?;
        Some((node.matrix?, node.row))
    }
}
