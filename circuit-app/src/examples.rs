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
];

pub fn by_id(id: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.id == id)
}
