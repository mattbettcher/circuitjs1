use std::env;
use std::fs;
use std::process;

use circuit_engine::{parse_dump, Circuit};

fn main() {
    let mut args = env::args().skip(1);
    let mut steps: usize = 1;
    let mut dc = false;
    let mut path: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--steps" => {
                steps = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| usage());
            }
            "--dc" => dc = true,
            "-h" | "--help" => usage(),
            p if p.starts_with('-') => {
                eprintln!("unknown flag {p}");
                usage();
            }
            p => path = Some(p.to_string()),
        }
    }
    let Some(path) = path else {
        usage();
    };
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("read {path}: {e}");
        process::exit(1);
    });
    let mut circuit: Circuit = parse_dump(&text).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    if dc {
        if let Err(e) = circuit.dc_operating_point() {
            eprintln!("{e}");
            process::exit(1);
        }
    } else if let Err(e) = circuit.steps(steps) {
        eprintln!("{e}");
        process::exit(1);
    }
    println!("t = {}", circuit.t());
    for n in 1..circuit.node_count() {
        println!("node {n} = {} V", circuit.node_voltage(n));
    }
}

fn usage() -> ! {
    eprintln!("usage: simulate [--dc] [--steps N] <circuit.txt>");
    process::exit(2);
}
