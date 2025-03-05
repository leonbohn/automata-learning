#![allow(missing_docs)]

use super::{Experiment, Experiments};
use automata::automaton::{MealyMachine, MooreMachine};
use automata::core::{
    Color, Void,
    alphabet::{Alphabet, Symbol},
    word::FiniteWord,
};
use automata::ts::{Deterministic, StateIndex, SymbolOf};
use automata::{DTS, Pointed, TransitionSystem};

pub trait Hypothesis: TransitionSystem + Deterministic + Pointed {
    type Output: Color;

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
        initial: StateIndex,
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

impl<A: Alphabet, Q: Color> Hypothesis for MooreMachine<A, Q, Void> {
    type Output = Q;

    fn output_from<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output {
        self.reached_state_color_from(source, input)
            .expect("Hypothesis must be complete")
    }
    fn from_transition_system(
        ts: DTS<Self::Alphabet, Self::StateColor, Self::EdgeColor>,
        initial: StateIndex,
    ) -> Self {
        Self::from_parts(ts, initial)
    }
    fn mandatory_experiments(
        alphabet: &Self::Alphabet,
    ) -> impl IntoIterator<Item = Experiment<SymbolOf<Self>>> {
        [Experiment::empty()]
    }
    fn give_state_color(
        mr: &[SymbolOf<Self>],
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
        source_mr: &[SymbolOf<Self>],
        a: SymbolOf<Self>,
        target_mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        source_row: &[Self::Output],
        target_row: &[Self::Output],
    ) -> Self::EdgeColor {
        assert!(!source_row.is_empty(), "cannot have zero experiments");
        Void
    }
}

impl<A: Alphabet, C: Color> Hypothesis for MealyMachine<A, Void, C> {
    type Output = C;

    fn output_from<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output {
        self.last_edge_color_from(source, input)
            .expect("Hypothesis must be complete")
    }
    fn from_transition_system(
        ts: DTS<Self::Alphabet, Self::StateColor, Self::EdgeColor>,
        initial: StateIndex,
    ) -> Self {
        Self::from_parts(ts, initial)
    }
    fn give_state_color(
        mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        row: &[Self::Output],
    ) -> Self::StateColor {
        Void
    }
    fn mandatory_experiments(
        alphabet: &Self::Alphabet,
    ) -> impl IntoIterator<Item = Experiment<SymbolOf<Self>>> {
        alphabet.universe().map(|a| Experiment(vec![a]))
    }
    fn give_transition_color(
        source_mr: &[SymbolOf<Self>],
        a: SymbolOf<Self>,
        target_mr: &[SymbolOf<Self>],
        experiments: &Experiments<Self>,
        source_row: &[Self::Output],
        target_row: &[Self::Output],
    ) -> Self::EdgeColor {
        let Some(i) = experiments.iter().position(|b| b.is_letter(a)) else {
            panic!("could not find experiment for {a:?}");
        };
        assert!(source_row.len() > i);
        source_row[i].clone()
    }
}
