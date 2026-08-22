/// One sample of node voltages and element branch values.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub t: f64,
    pub node_volts: Vec<f64>,
    pub elm_volts: Vec<Vec<f64>>,
    pub elm_currents: Vec<f64>,
}
