//! A library for learning automata from data.
#![allow(missing_docs)]
#![allow(unused)]

/// Contains passive learners such as RPNI, DBAInf and DPAInf.
#[macro_use]
pub mod passive;

/// Greedily LEarn Right Congruence algorithm, an algorithm that infers a
/// right congruence relation from a consistency function.
// pub mod glerc;
use automata::prelude::*;

// /// Deals with active learning algorithms such as L*.
pub mod active;
mod priority_mapping;

pub(crate) mod prefixtree;

#[cfg(test)]
mod tests {}
