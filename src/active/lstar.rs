use std::{cell::RefCell, fmt::Debug};

use automata::{prelude::*, word::Concat};
use fixedbitset::FixedBitSet;
use itertools::Itertools;
use tracing::{debug, info, trace};

use super::{oracle::Oracle, Experiment, ObservationTable};

const ITERATION_THRESHOLD: usize = if cfg!(debug_assertions) { 300 } else { 200000 };

pub trait LStarHypothesis: Deterministic + Sproutable + Pointed {
    type Color: Color;

    fn transform(&self, word: &[SymbolOf<Self>]) -> Self::Color;

    fn from_transition_system(
        ts: DTS<Self::Alphabet, Self::StateColor, Self::EdgeColor>,
        initial: StateIndex,
    ) -> Self;
    fn give_state_color(
        mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        row: &[Self::Color],
    ) -> Self::StateColor;

    fn give_transition_color(
        mr: &[SymbolOf<Self>],
        a: SymbolOf<Self>,
        experiments: &Experiments<Self>,
        row: &[Self::Color],
    ) -> Self::EdgeColor;

    fn mandatory_experiments(
        alphabet: &Self::Alphabet,
    ) -> impl IntoIterator<Item = Experiment<SymbolOf<Self>>>;
}

type Word<D> = Vec<SymbolOf<D>>;
pub type Experiments<D> = Vec<Experiment<SymbolOf<D>>>;

/// An implementation of the L* algorithm.
pub struct LStar<D: LStarHypothesis, T: Oracle<Alphabet = D::Alphabet>> {
    // the alphabet of what we are learning
    alphabet: D::Alphabet,
    // a mapping containing all queries that have been posed so far, together with their output
    queries: RefCell<math::Map<Word<D>, D::Color>>,
    // the minimal access words forming the base states
    base: Vec<Word<D>>,
    // all known experiments
    experiments: Vec<Experiment<SymbolOf<D>>>,
    // mapping from input word to a bitset, where the i-th entry gives the value for
    // the output of concatenating input word and i-th experiment
    table: math::Map<Word<D>, Vec<D::Color>>,
    // the oracle
    oracle: T,
    observations: ObservationTable<SymbolOf<D>, T::Output>,
}

impl<D: LStarHypothesis, T: Oracle<Alphabet = D::Alphabet, Output = D::Color>> LStar<D, T> {
    pub fn new(alphabet: D::Alphabet, oracle: T) -> Self {
        Self {
            experiments: D::mandatory_experiments(&alphabet).into_iter().collect(),
            observations: ObservationTable::with_rows_and_experiments(
                [],
                D::mandatory_experiments(&alphabet),
            ),
            alphabet,
            queries: RefCell::new(math::Map::default()),
            base: vec![vec![]],
            table: math::Map::default(),
            oracle,
        }
    }

    fn output(&self, w: &Word<D>) -> D::Color {
        if !self.queries.borrow().contains_key(w) {
            let c = self.oracle.output(w);
            assert!(self.queries.borrow_mut().insert(w.to_owned(), c).is_none());
        }
        self.queries.borrow().get(w).unwrap().clone()
    }

    fn update_table(&mut self) {
        let mut updates = vec![];
        let experiment_count = self.experiments.len();

        for (n, mr) in self.one_letter_extensions().enumerate() {
            let stored_experiment_count = self.table.get(&mr).map(|r| r.len()).unwrap_or(0);
            trace!("Have stored {stored_experiment_count} out of {experiment_count} experiments for {}", mr.as_string());
            if stored_experiment_count < experiment_count {
                for i in stored_experiment_count..experiment_count {
                    let concat = Concat(&mr, &self.experiments[i]).collect_vec();
                    let output = self.output(&concat).clone();
                    trace!(
                        "Adding update that {} maps to {:?}",
                        concat.as_string(),
                        output
                    );
                    updates.push((mr.clone(), i, output));
                }
            } else {
                assert_eq!(
                    stored_experiment_count,
                    experiment_count,
                    "Too many experiments present for {}",
                    mr.as_string()
                );
            }
        }

        for (mr, i, output) in updates {
            assert!(i < self.experiments.len());
            let mut row = self.table.entry(mr).or_default();
            row.push(output);
        }

        if cfg!(debug_assertions) {
            for mr in self.one_letter_extensions() {
                let Some(val) = self.table.get(&mr) else {
                    panic!("No table entry for {}", mr.as_string());
                };
                if val.len() != experiment_count {
                    panic!(
                        "Row for {} does not have enough entries, has {} of {experiment_count}",
                        mr.show(),
                        val.len()
                    )
                }
            }
        }

        trace!("After update the table is\n{:?}", self);
    }

    pub fn infer(&mut self) -> D {
        let start = std::time::Instant::now();
        let mut iteration = 0;
        let threshold = std::env::var("MAX_ITERATIONS")
            .unwrap_or(format!("{ITERATION_THRESHOLD}"))
            .parse()
            .unwrap();

        'outer: while iteration < threshold {
            self.update_table();
            iteration += 1;
            trace!("LStar iteration {iteration} with table\n{:?}", self);
            let todo = self.rows_to_promote();
            trace!(
                "Have to promote rows: {}",
                todo.iter().map(FiniteWord::as_string).join(", ")
            );

            if !todo.is_empty() {
                let mut queries = math::Set::default();
                for r in todo {
                    self.base.push(r.clone());
                    queries.insert(r.clone());
                }
                // TODO optimize this call!
                self.update_table();
                continue 'outer;
            }

            let hypothesis = self.hypothesis();

            // if let Err((counterexample, color)) = self.oracle.equivalence(&hypothesis) {
            //     assert!(hypothesis.transform(&counterexample) != color);
            //     self.process_counterexample(counterexample, color);
            //     continue 'outer;
            // }
            todo!();

            let duration = start.elapsed().as_millis();
            info!("Execution of LStar took {duration}ms");
            return hypothesis;
        }

        panic!("Iteration threshold exceeded!")
    }

    fn process_counterexample(&mut self, word: Word<D>, color: D::Color) {
        for i in 0..(word.len()) {
            let suffix = Experiment(word[i..].to_vec());
            assert!(!suffix.is_empty());
            if !self.experiments.contains(&suffix) {
                trace!("Adding counterexample {}", suffix.as_string());
                self.experiments.push(suffix)
            }
        }
    }

    fn state_color(&self, mr: &Word<D>) -> D::StateColor {
        D::give_state_color(
            mr,
            &self.experiments,
            self.table
                .get(mr)
                .expect("Cannot give color for non-existent word"),
        )
    }

    fn edge_color(&self, mr: &Word<D>, sym: SymbolOf<D>) -> D::EdgeColor {
        D::give_transition_color(mr, sym, &self.experiments, self.table.get(mr).unwrap())
    }

    fn hypothesis(&self) -> D {
        let start = std::time::Instant::now();

        let mut ts: DTS<_, _, _> = DTS::for_alphabet_size_hint(self.alphabet.clone(), 1);
        let mut state_map = math::Map::default();
        let mut observations = math::Map::default();

        for mr in &self.base {
            let color = self.state_color(mr);
            let id = ts.add_state(color);
            state_map.insert(mr, id);

            let observed = self.table.get(mr).unwrap();
            observations.insert(observed, mr);
        }

        for (mr, i) in &state_map {
            for a in self.alphabet.universe() {
                let color = self.edge_color(mr, a);
                let obs = self
                    .table
                    .get(&Concat(mr, [a]).into_vec())
                    .expect("Can only work if table is closed!");
                let target = observations.get(obs).unwrap();

                let added = ts.add_edge((
                    *i,
                    self.alphabet.make_expression(a),
                    color,
                    *state_map.get(target).unwrap(),
                ));
                assert!(added.is_some());
            }
        }

        let duration = start.elapsed().as_micros();
        debug!("Building hypothesis took {duration} microseconds");

        D::from_transition_system(ts, *state_map.get(&vec![]).unwrap())
    }

    fn inconsistent_words(&self) -> Option<Word<D>> {
        for (i, left) in self.base.iter().enumerate() {
            'middle: for right in &self.base[i..] {
                if self.table.get(left) != self.table.get(right) {
                    continue 'middle;
                }

                for sym in self.alphabet.universe() {
                    let left_ext = left
                        .iter()
                        .chain(std::iter::once(&sym))
                        .cloned()
                        .collect_vec();
                    let right_ext = right
                        .iter()
                        .chain(std::iter::once(&sym))
                        .cloned()
                        .collect_vec();

                    let l = self.table.get(&left_ext).expect("Should be present!");
                    let r = self.table.get(&right_ext).expect("Should be present!");

                    if l == r {
                        continue 'middle;
                    }
                    'inner: for (j, e) in self.experiments.iter().enumerate() {
                        assert!(j < std::cmp::min(l.len(), r.len()));
                        if l[j] == r[j] {
                            continue 'inner;
                        }
                        return Some(Concat(vec![sym], e).into_vec());
                    }
                }
            }
        }
        None
    }

    fn one_letter_extensions(&self) -> impl Iterator<Item = Word<D>> + '_ {
        self.base
            .iter()
            .flat_map(|w| {
                std::iter::once(w.clone()).chain(self.alphabet.universe().filter_map(|a| {
                    let mut x = w.clone();
                    x.push(a);
                    if !self.base.contains(&x) {
                        Some(x)
                    } else {
                        None
                    }
                }))
            })
            .unique()
    }

    fn rows_to_promote(&self) -> math::Set<Word<D>> {
        let start = std::time::Instant::now();

        let known = math::Set::from_iter(self.base.iter().map(|b| {
            self.table.get(b).unwrap_or_else(|| {
                panic!(
                    "Experiment {} must be present",
                    owo_colors::OwoColorize::blue(&b.as_string())
                )
            })
        }));
        let mut seen = math::Set::default();
        let mut out = math::Set::default();

        for word in self.one_letter_extensions() {
            trace!("Considering one letter extension {}", word.as_string());
            let seq = self.table.get(&word).unwrap();
            if !known.contains(seq) && seen.insert(seq) {
                out.insert(word);
            }
        }

        debug!(
            "Finding rows to promote took {} microseconds",
            start.elapsed().as_micros()
        );
        out
    }
}

impl<D: LStarHypothesis, T: Oracle<Alphabet = D::Alphabet>> std::fmt::Debug for LStar<D, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = tabled::builder::Builder::default();
        let mut header = vec!["MR".to_string()];

        for e in &self.experiments {
            header.push(e.as_string());
        }
        builder.push_record(header);

        for (i, mr) in self.base.iter().enumerate() {
            let mut row = vec![mr.as_string()];
            for color in self
                .table
                .get(mr)
                .unwrap_or_else(|| panic!("No table entry for {}", mr.as_string()))
            {
                row.push(format!("{:?}", color));
            }
            builder.push_record(row);
        }

        write!(f, "{}", builder.build())
    }
}

#[cfg(test)]
mod tests {}
