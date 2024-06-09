use automata::{prelude::*, transition_system::operations::MapStateColor};

use crate::passive::Sample;

use super::{Hypothesis, LStarHypothesis};

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

    fn output<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(&self, word: W) -> Self::Output;

    fn equivalence<H>(
        &self,
        hypothesis: H,
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
pub struct SampleOracle<A: Alphabet, W: LinearWord<A::Symbol>, C: Color> {
    sample: Sample<A, W, C>,
    default: C,
}

impl<A: Alphabet, X: FiniteWord<A::Symbol>, C: Color> Oracle for SampleOracle<A, X, C> {
    type Alphabet = A;

    type Output = C;

    fn output<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(&self, word: W) -> Self::Output {
        self.sample
            .words
            .iter()
            .find_map(|(k, v)| {
                if word.equals(k) {
                    Some(v.clone())
                } else {
                    None
                }
            })
            .unwrap_or(self.default.clone())
    }

    fn equivalence<H>(
        &self,
        hypothesis: H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        for (w, c) in &self.sample.words {
            if !hypothesis.output(w).eq(c) {
                return Err((w.into_vec(), c.clone()));
            }
        }
        Ok(())
    }

    fn alphabet(&self) -> &Self::Alphabet {
        self.sample.alphabet()
    }
}

impl<A: Alphabet, W: LinearWord<A::Symbol>, C: Color> SampleOracle<A, W, C> {
    /// Returns a reference to the underlying alphabet, as provided by [`Sample::alphabet()`].
    pub fn alphabet(&self) -> &A {
        self.sample.alphabet()
    }
}

impl<A: Alphabet, W: FiniteWord<A::Symbol>, C: Color> SampleOracle<A, W, C> {
    /// Creates a new instance of a [`SampleOracle`] with the given sample and default color.
    pub fn new(sample: Sample<A, W, C>, default: C) -> Self {
        Self { sample, default }
    }
}

impl<A: Alphabet, W: FiniteWord<A::Symbol>, C: Color> From<(Sample<A, W, C>, C)>
    for SampleOracle<A, W, C>
{
    fn from((value, default): (Sample<A, W, C>, C)) -> Self {
        Self::new(value, default)
    }
}

/// An oracle base on a [`DFALike`] instance. It answers membership queries by running the word through the
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

    fn output<W: FiniteWord<A::Symbol>>(&self, word: W) -> bool {
        (&self.automaton).into_dfa().accepts(word)
    }

    fn equivalence<H>(
        &self,
        hypothesis: H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        todo!()
    }
}

/// An oracle based on a [`MealyMachine`].
#[derive(Clone)]
pub struct MealyOracle<D: Congruence> {
    automaton: IntoMealyMachine<D>,
    default: Option<D::EdgeColor>,
}

impl<D> Oracle for MealyOracle<D>
where
    D: Congruence<Alphabet = CharAlphabet>,
    EdgeColor<D>: Color + Default,
{
    type Alphabet = D::Alphabet;
    type Output = EdgeColor<D>;

    fn output<W: FiniteWord<SymbolOf<D>>>(&self, word: W) -> D::EdgeColor {
        self.automaton
            .map(word)
            .or(self.default.clone())
            .expect("The oracle must be total!")
    }

    fn equivalence<H>(
        &self,
        hypothesis: H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        todo!()
    }

    fn alphabet(&self) -> &CharAlphabet {
        self.automaton.alphabet()
    }
}

impl<D> MealyOracle<D>
where
    D: Congruence,
    EdgeColor<D>: Color,
{
    /// Creates a new [`MealyOracle`] based on an instance of [`MealyLike`].
    pub fn new(automaton: D, default: Option<D::EdgeColor>) -> Self {
        Self {
            automaton: automaton.into_mealy(),
            default,
        }
    }

    pub fn alphabet(&self) -> &D::Alphabet {
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
    StateColor<D>: Color + Default,
{
    type Alphabet = D::Alphabet;
    type Output = StateColor<D>;

    fn alphabet(&self) -> &Self::Alphabet {
        self.automaton.alphabet()
    }

    fn output<W: FiniteWord<<Self::Alphabet as Alphabet>::Symbol>>(&self, word: W) -> Self::Output {
        todo!()
    }

    fn equivalence<H>(
        &self,
        hypothesis: H,
    ) -> Result<(), Counterexample<Self::Alphabet, Self::Output>>
    where
        H: Hypothesis<Alphabet = Self::Alphabet, Output = Self::Output>,
    {
        todo!()
    }
}

impl<D> MooreOracle<D>
where
    D: Congruence,
    EdgeColor<D>: Color,
{
    /// Creates a new [`MooreOracle`] based on an instance of [`MooreLike`].
    pub fn new(automaton: D) -> Self {
        Self { automaton }
    }
}

#[cfg(test)]
mod tests {
    use automata::prelude::*;

    use crate::active::LStar;

    use super::MealyOracle;

    #[test]
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
        // let mut learner = LStar::for_mealy(alphabet, oracle);
        // let mm = learner.infer();
        // assert_eq!(mm.size(), 2);
        todo!()
    }
}
