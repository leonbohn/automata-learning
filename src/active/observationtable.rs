use automata::prelude::*;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Experiment<S>(pub(super) Vec<S>);

impl<S: Symbol> FiniteWord<S> for Experiment<S> {
    type Symbols<'this> = std::iter::Cloned<std::slice::Iter<'this, S>>
    where
        Self: 'this;

    fn symbols(&self) -> Self::Symbols<'_> {
        self.0.iter().cloned()
    }
}

impl<S: Symbol> LinearWord<S> for Experiment<S> {
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
