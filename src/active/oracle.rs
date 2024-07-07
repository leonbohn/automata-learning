use std::cell::RefCell;

use automata::{prelude::*, transition_system::operations::MapStateColor};
use math::Set;
use tracing::trace;

use crate::passive::SetSample;

use super::Hypothesis;

pub type Counterexample<A, O> = (Vec<<A as Alphabet>::Symbol>, O);

/// A trait that encapsulates a minimally adequate teacher (MAT) for active learning. This is mainly used by
/// L*-esque algorithms and can be implemented by wildly different types, for example an automaton, a function
/// or even a collection of words.
///
/// This trait is designed in a generic way, allowing us to use it for learning a priority mapping, which assigns
/// non-empty finite words a value of type `Output`. This means we can learn a Mealy machine by using priorities as
/// the `Output` type, but it also enables us to learn a regular language/deterministic finite automaton by using
/// `bool` as the `Output` type.
pub trait Oracle {
    type Alphabet: Alphabet;
    type Output: Color;
    fn alphabet(&self) -> &Self::Alphabet;

    fn output<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        word: W,
    ) -> Self::Output;

    fn equivalence<H>(
        &self,
        hypothesis: &H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>;
}

pub fn lstar<H, O>(oracle: O) -> H
where
    O: Oracle,
    H: Hypothesis<Alphabet = O::Alphabet, Output = O::Output> + for<'a> From<&'a O::Alphabet>,
{
    oracle.alphabet().into()
}

/// An oracle/minimally adequate teacher based on a [`Sample`]. It answers membership queries by looking up the
/// word in the sample and returning the corresponding color. If the word is not in the sample, it returns the
/// default color. Equivalence queries are perfomed by checking if the hypothesis produces the same output as the
/// sample for all words in the sample.
#[derive(Debug, Clone)]
pub struct SampleOracle<A: Alphabet, W: Word<Symbol = A::Symbol>> {
    sample: SetSample<A, W>,
    default: bool,
}

impl<A: Alphabet, X: FiniteWord<Symbol = A::Symbol>> Oracle for SampleOracle<A, X> {
    type Alphabet = A;

    type Output = bool;

    fn output<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        word: W,
    ) -> Self::Output {
        todo!()
    }

    fn equivalence<H>(
        &self,
        hypothesis: &H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        todo!()
    }

    fn alphabet(&self) -> &Self::Alphabet {
        self.sample.alphabet()
    }
}

impl<A: Alphabet, W: Word<Symbol = A::Symbol>> SampleOracle<A, W> {
    /// Returns a reference to the underlying alphabet, as provided by [`Sample::alphabet()`].
    pub fn alphabet(&self) -> &A {
        self.sample.alphabet()
    }
}

impl<A: Alphabet, W: FiniteWord<Symbol = A::Symbol>> SampleOracle<A, W> {
    /// Creates a new instance of a [`SampleOracle`] with the given sample and default color.
    pub fn new(sample: SetSample<A, W>, default: bool) -> Self {
        Self { sample, default }
    }
}

impl<A: Alphabet, W: FiniteWord<Symbol = A::Symbol>> From<(SetSample<A, W>, bool)>
    for SampleOracle<A, W>
{
    fn from((value, default): (SetSample<A, W>, bool)) -> Self {
        Self::new(value, default)
    }
}

/// An oracle base on a [`DFA`] instance. It answers membership queries by running the word through the
/// automaton and returning the result. Equivalence queries are performed by intersecting the hypothesis with
/// the negated input automaton and returning a counterexample if the intersection is non-empty.
#[derive(Debug, Clone)]
pub struct DFAOracle<A: Alphabet> {
    automaton: DFA<A>,
    negated: DFA<A>,
}

impl<A: Alphabet> DFAOracle<A> {
    /// Creates a new instance of a [`DFAOracle`] from the given automaton.
    pub fn new(automaton: DFA<A>) -> Self {
        let negated = automaton.negation().collect_dfa();
        Self { negated, automaton }
    }
}

impl<A: Alphabet> Oracle for DFAOracle<A> {
    type Alphabet = A;
    type Output = bool;

    fn alphabet(&self) -> &Self::Alphabet {
        self.automaton.alphabet()
    }

    fn output<W: FiniteWord<Symbol = A::Symbol>>(&self, word: W) -> bool {
        self.automaton.accepts(word)
    }

    fn equivalence<H>(
        &self,
        hypothesis: &H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        for mr in (&self.automaton)
            .ts_product(hypothesis)
            .minimal_representatives()
        {
            match (self.automaton.accepts(&mr), hypothesis.output(&mr)) {
                (b, bb) if b != bb => return Err((mr.to_vec(), b)),
                _ => (),
            }
        }
        Ok(())
    }
}

/// An oracle based on a [`MealyMachine`].
#[derive(Clone)]
pub struct MealyOracle<A: Alphabet, C: Color = Int> {
    automaton: MealyMachine<A, Void, C>,
    default: Option<C>,
    missed: RefCell<Set<Vec<A::Symbol>>>,
}

impl<A: Alphabet, C: Color> MealyOracle<A, C> {
    /// Creates a new [`MealyOracle`] based on an instance of [`MealyMachine`].
    pub fn new(
        automaton: impl Congruence<Alphabet = A, EdgeColor = C>,
        default: Option<C>,
    ) -> Self {
        Self {
            automaton: automaton.erase_state_colors().collect_mealy(),
            default,
            missed: RefCell::new(Set::default()),
        }
    }
    pub fn alphabet(&self) -> &A {
        self.automaton.alphabet()
    }
}

impl<A: Alphabet, C: Color + Ord> Oracle for MealyOracle<A, C> {
    type Alphabet = A;
    type Output = C;

    fn output<W: FiniteWord<Symbol = A::Symbol>>(&self, word: W) -> C {
        self.automaton
            .last_edge_color(word)
            .or(self.default.clone())
            .expect("The oracle must be total!")
    }

    fn equivalence<H>(
        &self,
        hypothesis: &H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        for mr in (&self.automaton)
            .ts_product(hypothesis)
            .minimal_transition_representatives()
        {
            let Some(expected) = self.automaton.transform(&mr) else {
                continue;
            };
            // .unwrap_or_else(|| {
            //     let Some(default) = &self.default else {
            //         panic!("Oracle must be total or provide a default!")
            //     };
            // self.missed.borrow_mut().insert(mr.clone());
            // trace!("returning default for {mr:?}");
            // default.clone()
            // });
            let output = hypothesis.output(&mr);
            if output != expected {
                return Err((mr.to_vec(), expected));
            }
        }
        Ok(())
    }

    fn alphabet(&self) -> &A {
        self.automaton.alphabet()
    }
}

/// An oracle based on a [`MooreMachine`].
#[derive(Debug, Clone)]
pub struct MooreOracle<D> {
    automaton: D,
}

impl<D: Congruence> Oracle for MooreOracle<IntoMooreMachine<D>>
where
    StateColor<D>: Color + Default + Ord,
{
    type Alphabet = D::Alphabet;
    type Output = StateColor<D>;

    fn alphabet(&self) -> &Self::Alphabet {
        self.automaton.alphabet()
    }

    fn output<W: FiniteWord<Symbol = <Self::Alphabet as Alphabet>::Symbol>>(
        &self,
        word: W,
    ) -> Self::Output {
        self.automaton
            .reached_state_color(word)
            .expect("underlying transition system of Moore oracle must be complete")
    }

    fn equivalence<H>(
        &self,
        hypothesis: &H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        for mr in (&self.automaton)
            .ts_product(hypothesis)
            .minimal_representatives()
        {
            match (
                self.automaton
                    .transform(&mr)
                    .expect("source automaton must be complete"),
                hypothesis.output(&mr),
            ) {
                (c, cc) if c != cc => return Err((mr.to_vec(), c)),
                _ => (),
            }
        }
        Ok(())
    }
}

impl<D> MooreOracle<D>
where
    D: Congruence,
    EdgeColor<D>: Color,
{
    /// Creates a new [`MooreOracle`] based on an instance of something that behaves like a [`MooreMachine`].
    pub fn new(automaton: D) -> Self {
        Self { automaton }
    }
}

#[cfg(test)]
mod tests {
    use automata::prelude::*;

    use crate::active::LStar;

    use super::MealyOracle;

    #[test_log::test]
    fn mealy_al() {
        let target = DTS::builder()
            .with_transitions([
                (0, 'a', 1, 1),
                (0, 'b', 1, 0),
                (0, 'c', 1, 0),
                (1, 'a', 0, 0),
                (1, 'b', 1, 0),
                (1, 'c', 1, 0),
            ])
            .into_mealy(0);
        let oracle = MealyOracle::new(target, Some(0));
        let alphabet = oracle.alphabet().clone();
        let mut learner = LStar::new(alphabet, oracle);
        let mm: MealyMachine = learner.infer();
        assert_eq!(mm.size(), 2);
    }
}
