//! CircuitJS1-style modified nodal analysis engine.
//!
//! Transient simulation with trapezoidal / backward-Euler companion models,
//! Newton–Raphson linearization, and Crout LU. Licensed GPL-2.0-or-later
//! as a port of CircuitJS1.

mod circuit;
mod context;
mod dump;
mod element;
pub mod elements;
mod error;
mod geom;
mod lu;
mod matrix;
mod ports;
mod snapshot;

pub use circuit::Circuit;
pub use dump::parse_dump;
pub use element::{Element, ElementKind};
pub use error::{Result, SimError};
pub use lu::{lu_factor_dense, lu_solve_dense};
pub use ports::FLAG_BACK_EULER;
pub use snapshot::Snapshot;
