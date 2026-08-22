use crate::context::SimContext;

/// Circuit element that can stamp into the MNA system.
pub trait Element {
    fn post_count(&self) -> usize {
        self.posts().len()
    }
    fn posts(&self) -> &[(i32, i32)];
    fn internal_node_count(&self) -> usize {
        0
    }
    fn node_count(&self) -> usize {
        self.post_count() + self.internal_node_count()
    }
    fn voltage_source_count(&self) -> usize {
        0
    }
    fn is_removable_wire(&self) -> bool {
        false
    }
    fn connected_post(&self, post: usize) -> Option<(i32, i32)> {
        let _ = post;
        None
    }
    fn has_ground_connection(&self, _post: usize) -> bool {
        false
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn non_linear(&self) -> bool {
        false
    }
    fn is_ground(&self) -> bool {
        false
    }
    fn is_independent_voltage(&self) -> bool {
        false
    }

    fn set_node(&mut self, post: usize, node: usize);
    fn node(&self, post: usize) -> usize;
    fn set_voltage_source(&mut self, n: usize, vs: usize) {
        let _ = (n, vs);
    }
    fn vs_nodes(&self, _local: usize) -> (usize, usize) {
        (self.node(0), self.node(1))
    }
    fn set_node_voltage(&mut self, n: usize, v: f64);
    fn volts(&self) -> &[f64];
    fn current(&self) -> f64;
    fn set_current(&mut self, _vs: usize, c: f64) {
        let _ = c;
    }

    fn stamp(&mut self, ctx: &mut SimContext);
    fn start_iteration(&mut self, _ctx: &mut SimContext) {}
    fn do_step(&mut self, _ctx: &mut SimContext) {}
    fn step_finished(&mut self, _ctx: &mut SimContext) {}
    fn reset(&mut self) {}
    fn calculate_current(&mut self) {}
    fn prepare(&mut self, _dc_analysis: bool) {}
}
