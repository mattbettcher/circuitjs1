//! Built-in CircuitJS1 dumps matching the engine tests.

pub struct Example {
    pub id: &'static str,
    pub name: &'static str,
    pub dump: &'static str,
}

pub const EXAMPLES: &[Example] = &[
    Example {
        id: "divider",
        name: "Voltage divider",
        dump: "\
v 0 100 0 0 0 0 40 10 0 0 0.5
r 0 0 100 0 0 1000
r 100 0 100 100 0 1000
w 100 100 0 100 0
",
    },
    Example {
        id: "rc",
        name: "RC step",
        dump: "\
$ 0 0.000001
v 0 100 0 0 0 0 40 1 0 0 0.5
r 0 0 100 0 0 1000
c 100 0 100 100 0 0.000001 0 0 0
w 100 100 0 100 0
",
    },
    Example {
        id: "lc",
        name: "LC ring (trap)",
        dump: "\
$ 0 0.0000001
g 0 0 0 16 0
c 0 0 100 0 0 0.000001 1 1 0
l 100 0 0 0 0 0.001 0 0
",
    },
    Example {
        id: "diode",
        name: "Diode + resistor",
        dump: "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
d 100 0 100 100 0
w 100 100 0 100 0
",
    },
    Example {
        id: "current",
        name: "Current source",
        dump: "\
i 0 100 0 0 0 0.002
r 0 0 0 100 0 2500
g 0 100 0 116 0
",
    },
    Example {
        id: "islands",
        name: "Two islands",
        dump: "\
v 0 100 0 0 0 0 40 10 0 0 0.5
r 0 0 0 100 0 1000
v 200 100 200 0 0 0 40 3 0 0 0.5
r 200 0 200 100 0 1000
g 0 100 0 116 0
g 200 100 200 116 0
",
    },
    Example {
        id: "switch",
        name: "Switch + divider",
        dump: "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
s 100 0 100 100 0 0 false
w 100 100 0 100 0
",
    },
    Example {
        id: "rail",
        name: "Voltage rail",
        dump: "\
R 0 0 0 16 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
g 100 0 100 16 0
",
    },
    Example {
        id: "pot",
        name: "Potentiometer",
        dump: "\
v 0 100 0 0 0 0 40 10 0 0 0.5
g 0 100 0 116 0
174 0 0 100 0 0 1000 0.5 Resistance
g 100 0 100 16 0
",
    },
    Example {
        id: "opamp",
        name: "Op-amp follower",
        dump: "\
v 48 64 48 32 0 0 40 2 0 0 0.5
g 48 64 48 80 0
a 48 16 80 16 8 15 -15 1000000 0 0 100000
w 80 16 48 0 0
",
    },
    Example {
        id: "zener",
        name: "Zener shunt",
        dump: "\
v 0 100 0 0 0 0 40 10 0 0 0.5
r 0 0 100 0 0 1000
z 100 0 100 100 0 5.6
w 100 100 0 100 0
",
    },
    Example {
        id: "aswitch",
        name: "Analog switch",
        dump: "\
v 0 100 0 0 0 0 40 5 0 0 0.5
r 0 0 100 0 0 1000
159 100 0 100 100 0 20 10000000000 2.5
w 100 100 0 100 0
R 84 50 84 34 0 0 40 5 0 0 0.5
g 0 100 0 116 0
",
    },
    Example {
        id: "xfmr",
        name: "Transformer",
        dump: "\
v 0 64 0 16 0 1 40 5 0 0 0.5
r 0 16 64 16 0 100
T 64 16 128 48 0 4 1 0 0 0.999
r 64 48 0 48 0 100
g 0 64 0 80 0
r 128 16 192 16 0 100
r 128 48 192 48 0 100
w 192 16 192 48 0
",
    },
    Example {
        id: "schmitt",
        name: "Inverting Schmitt",
        dump: "\
R 0 0 0 16 0 0 40 0 0 0 0.5
g 0 16 0 32 0
183 0 0 64 0 0 0.5 1.66 3.33 5 0
",
    },
    Example {
        id: "and",
        name: "AND gate",
        dump: "\
g 0 80 0 96 0
L 32 16 16 16 0 1 false 5 0
L 32 48 16 48 0 1 false 5 0
150 32 32 96 32 0 2 0 5
",
    },
    Example {
        id: "inverter",
        name: "Inverter",
        dump: "\
g 0 32 0 48 0
L 0 0 16 0 0 1 false 5 0
I 0 0 64 0 0 0.5 5
",
    },
    Example {
        id: "dff",
        name: "D flip-flop",
        dump: "\
g 160 80 160 96 0
L 0 0 16 0 0 1 false 5 0
L 0 32 16 32 0 0 false 5 0
155 0 0 64 48 0
",
    },
    Example {
        id: "halfadd",
        name: "Half adder",
        dump: "\
g 0 80 0 96 0
L 0 0 16 0 0 1 false 5 0
L 0 32 16 32 0 1 false 5 0
195 0 0 64 32 0
",
    },
    Example {
        id: "mux",
        name: "2:1 mux",
        dump: "\
g 0 128 0 144 0
L 0 0 16 0 0 1 false 5 0
L 0 32 16 32 0 0 false 5 0
L 64 96 64 112 0 0 false 5 0
184 0 0 64 48 0 1
",
    },
    Example {
        id: "counter",
        name: "4-bit counter",
        dump: "\
g 160 128 160 144 0
L 0 0 16 0 0 0 false 5 0
L 0 96 16 96 0 1 false 5 0
164 0 0 64 48 0 4 0 0 0 0 true 0
",
    },
    Example {
        id: "sipo",
        name: "4-bit SIPO",
        dump: "\
g 160 96 160 112 0
L 0 32 16 32 0 1 false 5 0
L 0 64 16 64 0 0 false 5 0
189 0 0 64 48 0 4
",
    },
    Example {
        id: "decoder",
        name: "7-seg decoder (1)",
        dump: "\
g 160 240 160 256 0
L 0 96 16 96 0 1 false 5 0
197 0 0 64 48 0
",
    },
    Example {
        id: "seqgen",
        name: "Sequence generator",
        dump: "\
g 160 64 160 80 0
L 0 0 16 0 0 0 false 5 0
188 0 0 64 48 10 8 1
",
    },
];

pub fn by_id(id: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.id == id)
}
