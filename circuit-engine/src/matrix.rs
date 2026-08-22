use crate::lu::{lu_factor_dense, lu_solve_dense};

/// One electrically separate island (MNA system).
#[derive(Clone, Debug)]
pub struct CircuitMatrix {
    pub a: Vec<Vec<f64>>,
    pub rhs: Vec<f64>,
    pub orig_a: Vec<Vec<f64>>,
    pub orig_rhs: Vec<f64>,
    pub permute: Vec<i32>,
    pub size: usize,
    pub node_count: usize,
    pub non_linear: bool,
    pub node_voltages: Vec<f64>,
    pub last_node_voltages: Vec<f64>,
    pub node_ids: Vec<usize>,
}

impl CircuitMatrix {
    pub fn new() -> Self {
        Self {
            a: Vec::new(),
            rhs: Vec::new(),
            orig_a: Vec::new(),
            orig_rhs: Vec::new(),
            permute: Vec::new(),
            size: 0,
            node_count: 0,
            non_linear: false,
            node_voltages: Vec::new(),
            last_node_voltages: Vec::new(),
            node_ids: Vec::new(),
        }
    }

    pub fn alloc(&mut self) {
        let n = self.size;
        self.a = vec![vec![0.0; n]; n];
        self.rhs = vec![0.0; n];
        self.orig_a = vec![vec![0.0; n]; n];
        self.orig_rhs = vec![0.0; n];
        self.permute = vec![0; n];
        self.node_voltages = vec![0.0; self.node_count];
        if self.last_node_voltages.len() != self.node_count {
            self.last_node_voltages = vec![0.0; self.node_count];
        }
    }

    pub fn save_orig(&mut self) {
        let n = self.size;
        self.orig_rhs.copy_from_slice(&self.rhs);
        for i in 0..n {
            self.orig_a[i].copy_from_slice(&self.a[i]);
        }
    }

    pub fn restore_rhs(&mut self) {
        self.rhs.copy_from_slice(&self.orig_rhs);
    }

    pub fn restore_a(&mut self) {
        let n = self.size;
        for i in 0..n {
            self.a[i].copy_from_slice(&self.orig_a[i]);
        }
    }

    pub fn factor(&mut self) -> bool {
        lu_factor_dense(&mut self.a, &mut self.permute)
    }

    pub fn solve(&mut self) {
        lu_solve_dense(&self.a, &self.permute, &mut self.rhs);
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub index: usize,
    /// 0-based row in [`CircuitMatrix::a`]; unused for ground.
    pub row: usize,
    pub matrix: Option<usize>,
    pub internal: bool,
    pub links: Vec<NodeLink>,
}

#[derive(Clone, Copy, Debug)]
pub struct NodeLink {
    pub elm: usize,
    pub post: usize,
}

#[derive(Clone, Debug)]
pub struct VoltageSource {
    pub elm: usize,
    pub local: usize,
    pub matrix: usize,
    /// 0-based row in the island matrix.
    pub row: usize,
    pub n1: usize,
    pub n2: usize,
}
