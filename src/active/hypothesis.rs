#![allow(missing_docs)]

use super::{Experiment, Experiments};
use std::hash::Hash;
use automata::automaton::{MealyMachine, MooreMachine};
use automata::ts::{Deterministic, StateId, SymbolOf};
use automata::{
    Color, Void,
    alphabet::Alphabet,
    word::FiniteWord,
};
use automata::{DTS, Pointed, TransitionSystem};

pub trait Hypothesis: TransitionSystem + Deterministic + Pointed {
    type Output: Color + Eq + Hash;

    fn output_from<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output;
    fn output<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
    ) -> Self::Output {
        self.output_from(input, self.initial())
    }

    fn from_transition_system(
        ts: DTS<Self::Alphabet, Self::StateColor, Self::EdgeColor>,
        initial: StateId,
    ) -> Self;
    fn give_state_color(
        mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        row: &[Self::Output],
    ) -> Self::StateColor;

    fn give_transition_color(
        source_mr: &[SymbolOf<Self>],
        a: SymbolOf<Self>,
        target_mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        source_row: &[Self::Output],
        target_row: &[Self::Output],
    ) -> Self::EdgeColor;

    fn mandatory_experiments(
        alphabet: &Self::Alphabet,
    ) -> impl IntoIterator<Item = Experiment<SymbolOf<Self>>>;
}

impl<A: Alphabet, Q: Color + Eq + Hash> Hypothesis for MooreMachine<DTS<A, Q, Void>> {
    type Output = Q;

    fn output_from<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output {
        self.run_from(source, input)
            .reached_state_color()
            .expect("Hypothesis must be complete")
    }
    fn from_transition_system(
        ts: DTS<Self::Alphabet, Self::StateColor, Self::EdgeColor>,
        initial: StateId,
    ) -> Self {
        Self::from_parts(ts, initial)
    }
    fn mandatory_experiments(
        _alphabet: &Self::Alphabet,
    ) -> impl IntoIterator<Item = Experiment<SymbolOf<Self>>> {
        [Experiment::empty()]
    }
    fn give_state_color(
        _mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        row: &[Self::Output],
    ) -> Self::StateColor {
        debug_assert!(
            experiments
                .first()
                .expect("we need at least one experiment")
                .0
                .is_empty(),
            "first experiment should be empty word!"
        );
        row[0].clone()
    }
    fn give_transition_color(
        _source_mr: &[SymbolOf<Self>],
        _a: SymbolOf<Self>,
        _target_mr: &[SymbolOf<Self>],
        _experiments: &Experiments<Self>,
        source_row: &[Self::Output],
        _target_row: &[Self::Output],
    ) -> Self::EdgeColor {
        assert!(!source_row.is_empty(), "cannot have zero experiments");
        Void
    }
}

impl<A: Alphabet, C: Color + Eq + Hash> Hypothesis for MealyMachine<DTS<A, Void, C>> {
    type Output = C;

    fn output_from<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output {
        self.run_from(source, input)
            .last_edge_color()
            .expect("Hypothesis must be complete")
    }
    fn from_transition_system(
        ts: DTS<Self::Alphabet, Self::StateColor, Self::EdgeColor>,
        initial: StateId,
    ) -> Self {
        Self::from_parts(ts, initial)
    }
    fn give_state_color(
        _mr: &[SymbolOf<Self>],
        _experiments: &Experiments<Self>,
        _row: &[Self::Output],
    ) -> Self::StateColor {
        Void
    }
    fn mandatory_experiments(
        alphabet: &Self::Alphabet,
    ) -> impl IntoIterator<Item = Experiment<SymbolOf<Self>>> {
        alphabet.universe().map(|a| Experiment(vec![a]))
    }
    fn give_transition_color(
        _source_mr: &[SymbolOf<Self>],
        a: SymbolOf<Self>,
        _target_mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        source_row: &[Self::Output],
        _target_row: &[Self::Output],
    ) -> Self::EdgeColor {
        let Some(i) = experiments.iter().position(|b| b.is_letter(a)) else {
            panic!("could not find experiment for {a:?}");
        };
        assert!(source_row.len() > i);
        source_row[i].clone()
    }
}
