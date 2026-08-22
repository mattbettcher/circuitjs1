use crate::context::SimContext;

/// Schematic / inspector kind for an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Resistor,
    Capacitor,
    Inductor,
    Voltage,
    Rail,
    Current,
    Diode,
    Zener,
    Led,
    Wire,
    Ground,
    Switch,
    SwitchSpdt,
    LabeledNode,
    Pot,
    Probe,
    Output,
    TestPoint,
    Ammeter,
    OpAmp,
    Vcvs,
    Vccs,
    Transistor,
    Mosfet,
    Jfet,
    Transformer,
    TappedTransformer,
    AnalogSwitch,
    AnalogSwitchSpdt,
    Gyrator,
    Fuse,
    SparkGap,
    Memristor,
    Ldr,
    Thermistor,
    Relay,
    Schmitt,
    InvertingSchmitt,
    Ccvs,
    Cccs,
    Ccii,
    Varactor,
    Crystal,
    Sweep,
    AmSource,
    FmSource,
    Antenna,
    LogicInput,
    LogicOutput,
    Inverter,
    AndGate,
    NandGate,
    OrGate,
    NorGate,
    XorGate,
    XnorGate,
    DFlipFlop,
    JkFlipFlop,
    TFlipFlop,
    HalfAdder,
    FullAdder,
    Latch,
    Multiplexer,
    Demultiplexer,
}

impl ElementKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Resistor => "resistor",
            Self::Capacitor => "capacitor",
            Self::Inductor => "inductor",
            Self::Voltage => "voltage source",
            Self::Rail => "voltage rail",
            Self::Current => "current source",
            Self::Diode => "diode",
            Self::Zener => "zener",
            Self::Led => "LED",
            Self::Wire => "wire",
            Self::Ground => "ground",
            Self::Switch => "switch",
            Self::SwitchSpdt => "SPDT switch",
            Self::LabeledNode => "labeled node",
            Self::Pot => "potentiometer",
            Self::Probe => "probe",
            Self::Output => "output",
            Self::TestPoint => "test point",
            Self::Ammeter => "ammeter",
            Self::OpAmp => "op-amp",
            Self::Vcvs => "VCVS",
            Self::Vccs => "VCCS",
            Self::Transistor => "transistor",
            Self::Mosfet => "MOSFET",
            Self::Jfet => "JFET",
            Self::Transformer => "transformer",
            Self::TappedTransformer => "tapped transformer",
            Self::AnalogSwitch => "analog switch",
            Self::AnalogSwitchSpdt => "analog SPDT",
            Self::Gyrator => "gyrator",
            Self::Fuse => "fuse",
            Self::SparkGap => "spark gap",
            Self::Memristor => "memristor",
            Self::Ldr => "photoresistor",
            Self::Thermistor => "thermistor",
            Self::Relay => "relay",
            Self::Schmitt => "Schmitt trigger",
            Self::InvertingSchmitt => "inverting Schmitt",
            Self::Ccvs => "CCVS",
            Self::Cccs => "CCCS",
            Self::Ccii => "CCII",
            Self::Varactor => "varactor",
            Self::Crystal => "crystal",
            Self::Sweep => "AC sweep",
            Self::AmSource => "AM source",
            Self::FmSource => "FM source",
            Self::Antenna => "antenna",
            Self::LogicInput => "logic input",
            Self::LogicOutput => "logic output",
            Self::Inverter => "inverter",
            Self::AndGate => "AND gate",
            Self::NandGate => "NAND gate",
            Self::OrGate => "OR gate",
            Self::NorGate => "NOR gate",
            Self::XorGate => "XOR gate",
            Self::XnorGate => "XNOR gate",
            Self::DFlipFlop => "D flip-flop",
            Self::JkFlipFlop => "JK flip-flop",
            Self::TFlipFlop => "T flip-flop",
            Self::HalfAdder => "half adder",
            Self::FullAdder => "adder",
            Self::Latch => "latch",
            Self::Multiplexer => "multiplexer",
            Self::Demultiplexer => "demultiplexer",
        }
    }
}

/// Circuit element that can stamp into the MNA system.
pub trait Element {
    fn post_count(&self) -> usize {
        self.posts().len()
    }
    fn posts(&self) -> &[(i32, i32)];
    /// Terminals used for drawing (ground keeps a second point for the symbol).
    fn geometry(&self) -> &[(i32, i32)] {
        self.posts()
    }
    fn kind(&self) -> ElementKind;
    /// Primary parameter, if any, as `(value, SI unit)` e.g. `(1000.0, "Ω")`.
    fn primary_value(&self) -> Option<(f64, &'static str)> {
        None
    }
    fn tag(&self) -> &'static str {
        ""
    }
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
    /// Same-matrix coupling (gate↔channel, op-amp terminals, controlled sources).
    fn get_matrix_connection(&self, n1: usize, n2: usize) -> bool {
        self.get_connection(n1, n2)
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
    /// Shared net name for labeled nodes.
    fn net_name(&self) -> Option<&str> {
        None
    }
    /// Toggle switch/state. Return true if topology must be re-analyzed.
    fn toggle(&mut self) -> bool {
        false
    }
    fn position(&self) -> i32 {
        0
    }
    fn set_position(&mut self, _p: i32) {}
    fn set_slider(&mut self, _t: f64) {}

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
