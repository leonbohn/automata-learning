use automata::core::alphabet::Symbol;
use automata::core::math;
use automata::core::word::{FiniteWord, Word};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Experiment<S>(pub(super) Vec<S>);

impl<S> Experiment<S> {
    pub fn empty() -> Self {
        Self(vec![])
    }
    pub fn is_letter(&self, letter: S) -> bool
    where
        S: Symbol,
    {
        self.0 == [letter]
    }
}

impl<S: Symbol> FiniteWord for Experiment<S> {
    type Symbols<'this>
        = std::iter::Cloned<std::slice::Iter<'this, S>>
    where
        Self: 'this;

    fn symbols(&self) -> Self::Symbols<'_> {
        self.0.iter().cloned()
    }
}

impl<S: Symbol> Word for Experiment<S> {
    type Symbol = S;
    const FINITE: bool = true;
    fn nth(&self, position: usize) -> Option<S> {
        self.0.get(position).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Representative<S>(pub(super) Vec<S>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRow<X>(pub(super) Vec<X>);

pub struct ObservationTable<S, X> {
    pub(crate) experiments: Vec<Experiment<S>>,
    pub(crate) outputs: math::Map<Representative<S>, OutputRow<X>>,
}

impl<S, X> Default for ObservationTable<S, X>
where
    S: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, X> ObservationTable<S, X>
where
    S: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            experiments: vec![],
            outputs: math::Map::default(),
        }
    }

    pub fn with_rows_and_experiments<I, J>(rows: I, experiments: J) -> Self
    where
        I: IntoIterator<Item = Representative<S>>,
        J: IntoIterator<Item = Experiment<S>>,
    {
        Self {
            experiments: experiments.into_iter().collect(),
            outputs: rows.into_iter().map(|r| (r, vec![].into())).collect(),
        }
    }
}

impl<X> From<Vec<X>> for OutputRow<X> {
    fn from(value: Vec<X>) -> Self {
        Self(value)
    }
}
