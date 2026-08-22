use std::collections::{HashMap, VecDeque};

use crate::context::SimContext;
use crate::element::Element;
use crate::error::{Result, SimError};
use crate::matrix::{CircuitMatrix, Node, NodeLink, VoltageSource};

/// Headless CircuitJS1-style transient MNA engine.
pub struct Circuit {
    pub elements: Vec<Box<dyn Element>>,
    ctx: SimContext,
    unconnected: Vec<usize>,
    circuit_nonlinear: bool,
    pub max_time_step: f64,
    pub min_time_step: f64,
    pub adjust_time_step: bool,
    good_iterations: i32,
    analyzed: bool,
    stamped: bool,
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            ctx: SimContext {
                matrices: Vec::new(),
                nodes: Vec::new(),
                voltage_sources: Vec::new(),
                time_step: 5e-6,
                t: 0.0,
                dc_analysis: false,
                converged: true,
                sub_iterations: 0,
            },
            unconnected: Vec::new(),
            circuit_nonlinear: false,
            max_time_step: 5e-6,
            min_time_step: 50e-12,
            adjust_time_step: false,
            good_iterations: 100,
            analyzed: false,
            stamped: false,
        }
    }

    pub fn push(&mut self, elm: Box<dyn Element>) -> usize {
        let i = self.elements.len();
        self.elements.push(elm);
        self.analyzed = false;
        self.stamped = false;
        i
    }

    pub fn t(&self) -> f64 {
        self.ctx.t
    }

    pub fn time_step(&self) -> f64 {
        self.ctx.time_step
    }

    pub fn set_time_step(&mut self, dt: f64) {
        self.max_time_step = dt;
        self.ctx.time_step = dt;
        self.stamped = false;
    }

    pub fn set_adjust_time_step(&mut self, adjust: bool) {
        self.adjust_time_step = adjust;
    }

    pub fn set_dc_analysis(&mut self, dc: bool) {
        self.ctx.dc_analysis = dc;
        self.stamped = false;
    }

    pub fn node_count(&self) -> usize {
        self.ctx.nodes.len()
    }

    pub fn node_voltage(&self, n: usize) -> f64 {
        if n == 0 {
            return 0.0;
        }
        let node = &self.ctx.nodes[n];
        if let Some(mi) = node.matrix {
            self.ctx.matrices[mi].node_voltages[node.row]
        } else {
            0.0
        }
    }

    pub fn element_current(&self, i: usize) -> f64 {
        self.elements[i].current()
    }

    pub fn element_volts(&self, i: usize) -> &[f64] {
        self.elements[i].volts()
    }

    pub fn island_count(&self) -> usize {
        self.ctx.matrices.len()
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Voltages and currents at the current time.
    pub fn snapshot(&self) -> crate::snapshot::Snapshot {
        crate::snapshot::Snapshot {
            t: self.t(),
            node_volts: (0..self.node_count())
                .map(|n| self.node_voltage(n))
                .collect(),
            elm_volts: (0..self.elements.len())
                .map(|i| self.elements[i].volts().to_vec())
                .collect(),
            elm_currents: (0..self.elements.len())
                .map(|i| self.elements[i].current())
                .collect(),
        }
    }

    pub fn toggle(&mut self, i: usize) {
        if self.elements[i].toggle() {
            self.analyzed = false;
            self.stamped = false;
        }
    }

    pub fn set_slider(&mut self, i: usize, t: f64) {
        self.elements[i].set_slider(t);
        self.stamped = false;
    }

    pub fn reset(&mut self) {
        self.ctx.t = 0.0;
        self.ctx.time_step = self.max_time_step;
        self.good_iterations = 100;
        for e in &mut self.elements {
            e.reset();
        }
        self.stamped = false;
    }

    /// Build node lists, islands, and voltage-source rows.
    pub fn analyze(&mut self) -> Result<()> {
        if self.elements.is_empty() {
            return Err(SimError::EmptyCircuit);
        }

        for e in &mut self.elements {
            e.prepare(self.ctx.dc_analysis);
        }

        let mut uf = UnionFind::new();
        let mut ground_points: Vec<(i32, i32)> = Vec::new();
        let mut first_voltage_post: Option<(i32, i32)> = None;

        for e in &self.elements {
            if e.is_ground() {
                if let Some(&p) = e.posts().first() {
                    ground_points.push(p);
                    uf.find(p);
                }
            }
            if e.is_independent_voltage() && first_voltage_post.is_none() {
                first_voltage_post = e.posts().first().copied();
            }
            if e.is_removable_wire() && !e.is_ground() {
                for post in 0..e.post_count() {
                    if let Some(p1) = e.connected_post(post) {
                        let p0 = e.posts()[post];
                        uf.union(p0, p1);
                    }
                }
            }
        }

        let mut labels: HashMap<String, (i32, i32)> = HashMap::new();
        for e in &self.elements {
            if let Some(name) = e.net_name() {
                if let Some(&p0) = e.posts().first() {
                    if let Some(&first) = labels.get(name) {
                        uf.union(first, p0);
                    } else {
                        labels.insert(name.to_string(), p0);
                    }
                }
            }
        }

        let mut ground_root: Option<(i32, i32)> = None;
        if !ground_points.is_empty() {
            let g0 = ground_points[0];
            for &p in &ground_points[1..] {
                uf.union(g0, p);
            }
            ground_root = Some(uf.find(g0));
        } else if let Some(p) = first_voltage_post {
            ground_root = Some(uf.find(p));
        }

        // Allocate ground node 0.
        self.ctx.nodes.clear();
        self.ctx.nodes.push(Node {
            index: 0,
            row: 0,
            matrix: None,
            internal: false,
            links: Vec::new(),
        });

        let mut root_to_node: HashMap<(i32, i32), usize> = HashMap::new();
        if let Some(g) = ground_root {
            root_to_node.insert(uf.find(g), 0);
        }

        for ei in 0..self.elements.len() {
            let posts = self.elements[ei].post_count();
            for j in 0..posts {
                let pt = self.elements[ei].posts()[j];
                let root = uf.find(pt);
                let node_idx = if let Some(&n) = root_to_node.get(&root) {
                    n
                } else {
                    let n = self.ctx.nodes.len();
                    self.ctx.nodes.push(Node {
                        index: n,
                        row: 0,
                        matrix: None,
                        internal: false,
                        links: Vec::new(),
                    });
                    root_to_node.insert(root, n);
                    n
                };
                self.elements[ei].set_node(j, node_idx);
                self.ctx.nodes[node_idx]
                    .links
                    .push(NodeLink { elm: ei, post: j });
                if node_idx == 0 {
                    self.elements[ei].set_node_voltage(j, 0.0);
                }
            }
            let internals = self.elements[ei].internal_node_count();
            for j in 0..internals {
                let n = self.ctx.nodes.len();
                self.ctx.nodes.push(Node {
                    index: n,
                    row: 0,
                    matrix: None,
                    internal: true,
                    links: Vec::new(),
                });
                let post = posts + j;
                self.elements[ei].set_node(post, n);
                self.ctx.nodes[n].links.push(NodeLink { elm: ei, post });
            }
        }

        self.circuit_nonlinear = false;
        self.ctx.voltage_sources.clear();
        for ei in 0..self.elements.len() {
            if self.elements[ei].non_linear() {
                self.circuit_nonlinear = true;
            }
            let nvs = self.elements[ei].voltage_source_count();
            for local in 0..nvs {
                let idx = self.ctx.voltage_sources.len();
                let (n1, n2) = self.elements[ei].vs_nodes(local);
                self.elements[ei].set_voltage_source(local, idx);
                self.ctx.voltage_sources.push(VoltageSource {
                    elm: ei,
                    local,
                    matrix: 0,
                    row: 0,
                    n1,
                    n2,
                });
            }
        }

        self.find_unconnected_nodes();
        self.calculate_closures();

        let mut vs_per_matrix = vec![0usize; self.ctx.matrices.len()];
        for vs in &mut self.ctx.voltage_sources {
            let matrix = if vs.n1 != 0 {
                self.ctx.nodes[vs.n1].matrix
            } else {
                None
            }
            .or_else(|| {
                if vs.n2 != 0 {
                    self.ctx.nodes[vs.n2].matrix
                } else {
                    None
                }
            })
            .unwrap_or(0);
            vs.matrix = matrix;
            vs_per_matrix[matrix] += 1;
            vs.row = self.ctx.matrices[matrix].node_count + vs_per_matrix[matrix] - 1;
        }
        for (i, m) in self.ctx.matrices.iter_mut().enumerate() {
            m.size = m.node_count + vs_per_matrix[i];
        }

        self.ctx.time_step = self.max_time_step;
        self.analyzed = true;
        self.stamped = false;
        Ok(())
    }

    fn find_unconnected_nodes(&mut self) {
        let total = self.ctx.nodes.len();
        let mut closure = vec![false; total];
        self.unconnected.clear();
        if total > 0 {
            closure[0] = true;
        }
        for e in &self.elements {
            for j in 0..e.post_count() {
                if e.has_ground_connection(j) {
                    closure[e.node(j)] = true;
                }
            }
        }
        let mut queue: VecDeque<usize> = VecDeque::new();
        for (i, c) in closure.iter().enumerate() {
            if *c {
                queue.push_back(i);
            }
        }
        let mut scan_from = 1usize;
        loop {
            if let Some(n) = queue.pop_front() {
                for link in self.ctx.nodes[n].links.clone() {
                    let e = &self.elements[link.elm];
                    let post1 = link.post;
                    for k in 0..e.post_count() {
                        if k == post1 {
                            continue;
                        }
                        let kn = e.node(k);
                        if !closure[kn] && e.get_connection(post1, k) {
                            closure[kn] = true;
                            queue.push_back(kn);
                        }
                    }
                }
            } else {
                let mut found = false;
                while scan_from < total {
                    if !closure[scan_from] && !self.ctx.nodes[scan_from].internal {
                        self.unconnected.push(scan_from);
                        closure[scan_from] = true;
                        queue.push_back(scan_from);
                        scan_from += 1;
                        found = true;
                        break;
                    }
                    scan_from += 1;
                }
                if !found {
                    break;
                }
            }
        }
    }

    fn calculate_closures(&mut self) {
        let total = self.ctx.nodes.len();
        let mut closure_index = vec![-1i32; total];
        let mut closure_count = 0i32;
        for i in 1..total {
            if closure_index[i] >= 0 {
                continue;
            }
            let mut stack = vec![i];
            closure_index[i] = closure_count;
            while let Some(n) = stack.pop() {
                for link in self.ctx.nodes[n].links.clone() {
                    let e = &self.elements[link.elm];
                    let post1 = link.post;
                    for k in 0..e.node_count() {
                        if k == post1 {
                            continue;
                        }
                        if !e.get_matrix_connection(post1, k) {
                            continue;
                        }
                        let kn = e.node(k);
                        if kn == 0 {
                            continue;
                        }
                        if closure_index[kn] < 0 {
                            closure_index[kn] = closure_count;
                            stack.push(kn);
                        }
                    }
                }
            }
            closure_count += 1;
        }
        if closure_count == 0 {
            closure_count = 1;
        }
        self.ctx.matrices = (0..closure_count as usize)
            .map(|_| CircuitMatrix::new())
            .collect();
        for i in 1..total {
            let ci = closure_index[i];
            if ci < 0 {
                continue;
            }
            let mi = ci as usize;
            self.ctx.matrices[mi].node_count += 1;
            self.ctx.nodes[i].row = self.ctx.matrices[mi].node_count - 1;
            self.ctx.nodes[i].matrix = Some(mi);
            self.ctx.matrices[mi].node_ids.push(i);
        }
    }

    pub fn stamp(&mut self) -> Result<()> {
        if !self.analyzed {
            self.analyze()?;
        }
        for m in &mut self.ctx.matrices {
            m.alloc();
            m.non_linear = self.circuit_nonlinear;
        }
        for &n in &self.unconnected {
            self.ctx.stamp_resistor(0, n, 1e8);
        }
        for e in &mut self.elements {
            e.stamp(&mut self.ctx);
        }
        for m in &mut self.ctx.matrices {
            m.save_orig();
            if !m.non_linear && m.size > 0 && !m.factor() {
                return Err(SimError::SingularMatrix);
            }
        }
        self.stamped = true;
        Ok(())
    }

    fn ensure_stamped(&mut self) -> Result<()> {
        if !self.stamped {
            self.stamp()?;
        }
        Ok(())
    }

    /// One timestep, including Newton subiterations for nonlinear circuits.
    pub fn step(&mut self) -> Result<()> {
        loop {
            self.ensure_stamped()?;
            if self.adjust_time_step
                && self.good_iterations >= 3
                && self.ctx.time_step < self.max_time_step
            {
                self.ctx.time_step = (self.ctx.time_step * 2.0).min(self.max_time_step);
                self.stamped = false;
                self.stamp()?;
                self.good_iterations = 0;
            }

            for e in &mut self.elements {
                e.start_iteration(&mut self.ctx);
            }

            let subiter_count =
                if self.adjust_time_step && self.ctx.time_step / 2.0 > self.min_time_step {
                    100
                } else {
                    5000
                };

            let mut subiter = 0;
            while subiter != subiter_count {
                self.ctx.converged = true;
                self.ctx.sub_iterations = subiter;
                for m in &mut self.ctx.matrices {
                    m.restore_rhs();
                    if m.non_linear {
                        m.restore_a();
                    }
                }
                for e in &mut self.elements {
                    e.do_step(&mut self.ctx);
                }
                for mi in 0..self.ctx.matrices.len() {
                    if self.ctx.matrices[mi].size < 8 {
                        for row in &self.ctx.matrices[mi].a {
                            for &x in row {
                                if !x.is_finite() {
                                    return Err(SimError::Invalid("nan/infinite matrix!".into()));
                                }
                            }
                        }
                    }
                    if self.ctx.matrices[mi].non_linear {
                        if self.ctx.converged && subiter > 0 {
                            continue;
                        }
                        if !self.ctx.matrices[mi].factor() {
                            return Err(SimError::SingularMatrix);
                        }
                    }
                    self.ctx.matrices[mi].solve();
                    self.apply_solved(mi)?;
                }
                if !self.circuit_nonlinear {
                    break;
                }
                if self.ctx.converged && subiter > 0 {
                    break;
                }
                subiter += 1;
            }

            if subiter == subiter_count {
                self.good_iterations = 0;
                if self.adjust_time_step {
                    self.ctx.time_step /= 2.0;
                }
                if self.ctx.time_step < self.min_time_step || !self.adjust_time_step {
                    return Err(SimError::ConvergenceFailed);
                }
                self.restore_last_voltages();
                self.stamped = false;
                continue;
            }

            if subiter < 3 {
                self.good_iterations += 1;
            } else {
                self.good_iterations = 0;
            }

            self.ctx.t += self.ctx.time_step;
            for e in &mut self.elements {
                e.step_finished(&mut self.ctx);
            }
            for m in &mut self.ctx.matrices {
                m.last_node_voltages.copy_from_slice(&m.node_voltages);
            }
            return Ok(());
        }
    }

    fn apply_solved(&mut self, mi: usize) -> Result<()> {
        let size = self.ctx.matrices[mi].size;
        for j in 0..size {
            let res = self.ctx.matrices[mi].rhs[j];
            if res.is_nan() {
                self.ctx.converged = false;
                break;
            }
            if j < self.ctx.matrices[mi].node_count {
                self.ctx.matrices[mi].node_voltages[j] = res;
            }
        }
        let vs_updates: Vec<(usize, usize, f64)> = self
            .ctx
            .voltage_sources
            .iter()
            .filter(|vs| vs.matrix == mi)
            .map(|vs| (vs.elm, vs.local, self.ctx.matrices[mi].rhs[vs.row]))
            .collect();
        for (elm, local, c) in vs_updates {
            if c.is_finite() {
                self.elements[elm].set_current(local, c);
            }
        }
        self.set_node_voltages_from(mi, false);
        Ok(())
    }

    fn set_node_voltages_from(&mut self, mi: usize, last: bool) {
        let ids = self.ctx.matrices[mi].node_ids.clone();
        for nid in ids {
            let row = self.ctx.nodes[nid].row;
            let v = if last {
                self.ctx.matrices[mi].last_node_voltages[row]
            } else {
                self.ctx.matrices[mi].node_voltages[row]
            };
            let links = self.ctx.nodes[nid].links.clone();
            for link in links {
                self.elements[link.elm].set_node_voltage(link.post, v);
            }
        }
    }

    fn restore_last_voltages(&mut self) {
        for mi in 0..self.ctx.matrices.len() {
            let n = self.ctx.matrices[mi].node_count;
            for i in 0..n {
                self.ctx.matrices[mi].node_voltages[i] =
                    self.ctx.matrices[mi].last_node_voltages[i];
            }
            self.set_node_voltages_from(mi, true);
        }
    }

    pub fn steps(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            self.step()?;
        }
        Ok(())
    }

    pub fn run_to(&mut self, t_end: f64) -> Result<()> {
        while self.ctx.t + self.ctx.time_step * 0.5 < t_end {
            self.step()?;
        }
        Ok(())
    }

    /// DC operating point, then restamp for transient.
    pub fn dc_operating_point(&mut self) -> Result<()> {
        self.ctx.dc_analysis = true;
        self.analyzed = false;
        self.stamp()?;
        self.step()?;
        self.ctx.t = 0.0;
        self.ctx.dc_analysis = false;
        self.analyzed = false;
        self.stamp()?;
        Ok(())
    }
}

struct UnionFind {
    parent: HashMap<(i32, i32), (i32, i32)>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    fn find(&mut self, p: (i32, i32)) -> (i32, i32) {
        let par = *self.parent.entry(p).or_insert(p);
        if par != p {
            let root = self.find(par);
            self.parent.insert(p, root);
            root
        } else {
            p
        }
    }

    fn union(&mut self, a: (i32, i32), b: (i32, i32)) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent.insert(rb, ra);
        }
    }
}
