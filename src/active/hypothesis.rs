#![allow(missing_docs)]
use automata::prelude::*;

pub trait Hypothesis {
    type Alphabet: Alphabet;
    type Output: Color;
    type StateIndex: IndexType;

    fn initial(&self) -> Self::StateIndex;
    fn reached_index_from<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::StateIndex;
    fn reached_index<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
    ) -> Self::StateIndex {
        self.reached_index_from(input, self.initial())
    }
    fn output_from<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output;
    fn output<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
    ) -> Self::Output {
        self.output_from(input, self.initial())
    }
}

impl<A: Alphabet, Q: Color, C: Color> Hypothesis for MooreMachine<A, Q, C> {
    type Alphabet = A;
    type Output = Q;

    type StateIndex = StateIndex<Self>;

    fn initial(&self) -> Self::StateIndex {
        Pointed::initial(self)
    }

    fn reached_index_from<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::StateIndex {
        self.reached_state_index_from(source, input)
            .expect("Hypothesis must be complete")
    }

    fn output_from<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output {
        self.reached_state_color_from(source, input)
            .expect("Hypothesis must be complete")
    }
}

impl<A: Alphabet, Q: Color, C: Color> Hypothesis for MealyMachine<A, Q, C> {
    type Alphabet = A;

    type Output = C;

    type StateIndex = StateIndex<Self>;

    fn initial(&self) -> Self::StateIndex {
        Pointed::initial(self)
    }

    fn reached_index_from<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::StateIndex {
        self.reached_state_index_from(source, input)
            .expect("Hypothesis must be complete")
    }

    fn output_from<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        input: W,
        source: Self::StateIndex,
    ) -> Self::Output {
        self.last_edge_color_from(source, input)
            .expect("Hypothesis must be complete")
    }
}
