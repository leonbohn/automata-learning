use automata::Map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Experiment<S>(pub(super) Vec<S>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Representative<S>(pub(super) Vec<S>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRow<X>(pub(super) Vec<X>);

pub struct ObservationTable<S, X> {
    pub(crate) experiments: Vec<Experiment<S>>,
    pub(crate) outputs: Map<Representative<S>, OutputRow<X>>,
}

impl<S, X> ObservationTable<S, X> {
    pub fn new() -> Self {
        Self {
            experiments: vec![],
            outputs: Map::default(),
        }
    }

    pub fn with_rows_and_experiments<I, J>(rows: I, experiments: J) -> Self
    where
        I: IntoIterator<Item = Representative<S>>,
        J: IntoIterator<Item = Experiment<S>>,
    {
        Self {
            experiments: experiments.into_iter().collect(),
            outputs: rows.into_iter().map(|r| (r, vec![])).collect(),
        }
    }
}
