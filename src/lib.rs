//! A library for learning automata from data.
//!
//! **Deprecated:** this crate has moved into [`automata`](https://crates.io/crates/automata) (version 0.2.1
//! and later) as the module `automata::learning`, behind the feature `learning`. Use
//! `automata = { version = "0.2.1", features = ["learning"] }` instead; this crate is not developed any
//! further.
#![allow(missing_docs)]
#![allow(unused)]

/// Contains passive learners such as RPNI, DBAInf and DPAInf.
#[macro_use]
pub mod passive;

// /// Deals with active learning algorithms such as L*.
pub mod active;
mod priority_mapping;
pub use priority_mapping::{AnnotatedCongruence, Annotation, WeakPriorityMapping};

pub(crate) mod prefixtree;

#[cfg(test)]
mod tests {}
