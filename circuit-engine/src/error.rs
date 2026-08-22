use std::fmt;

/// Simulation or parse failure.
#[derive(Debug, Clone, PartialEq)]
pub enum SimError {
    SingularMatrix,
    ConvergenceFailed,
    EmptyCircuit,
    Parse(String),
    Invalid(String),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::SingularMatrix => write!(f, "Singular matrix!"),
            SimError::ConvergenceFailed => write!(f, "Convergence failed!"),
            SimError::EmptyCircuit => write!(f, "Empty circuit"),
            SimError::Parse(s) => write!(f, "parse error: {s}"),
            SimError::Invalid(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for SimError {}

pub type Result<T> = std::result::Result<T, SimError>;
