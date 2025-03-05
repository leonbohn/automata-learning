use crate::passive::dpainf::{dpainf, iteration_consistency_conflicts};
use automata::core::alphabet::Alphabet;
use automata::core::math;
use automata::ts::{Deterministic, StateIndex};
use automata::{families::FORC, Class, RightCongruence};
use itertools::Itertools;

use super::{OmegaSample, SetSample};

/// An [`OmegaSample`] restricted/split onto one [`Class`] of a [`RightCongruence`].
#[derive(Clone)]
pub struct ClassOmegaSample<'a, A: Alphabet> {
    congruence: &'a RightCongruence<A>,
    class: Class<A::Symbol>,
    sample: OmegaSample<A>,
}

impl<A: Alphabet> std::ops::Deref for ClassOmegaSample<'_, A> {
    type Target = OmegaSample<A>;

    fn deref(&self) -> &Self::Target {
        &self.sample
    }
}

impl<A: Alphabet> std::ops::DerefMut for ClassOmegaSample<'_, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sample
    }
}

impl<'a, A: Alphabet> ClassOmegaSample<'a, A> {
    /// Creates a new [`ClassOmegaSample`] from a [`RightCongruence`], a [`Class`] and a [`SetSample`].
    pub fn new(
        congruence: &'a RightCongruence<A>,
        class: Class<A::Symbol>,
        sample: OmegaSample<A>,
    ) -> Self {
        Self {
            congruence,
            class,
            sample,
        }
    }

    /// Returns a reference to the underlying sample.
    pub fn sample(&self) -> &OmegaSample<A> {
        &self.sample
    }

    /// Gives a mutable reference to the underlying sample.
    pub fn sample_mut(&mut self) -> &mut OmegaSample<A> {
        &mut self.sample
    }

    /// Creates an empty [`ClassOmegaSample`] from a [`RightCongruence`], a [`Class`] and an alphabet.
    pub fn empty(congruence: &'a RightCongruence<A>, class: Class<A::Symbol>, alphabet: A) -> Self {
        Self {
            congruence,
            class,
            sample: SetSample {
                alphabet,
                positive: math::Set::default(),
                negative: math::Set::default(),
            },
        }
    }
}

/// Represents a right congruence relation together with a collection of split samples, one
/// associated with each class of the congruence.
#[derive(Clone)]
pub struct SplitOmegaSample<'a, A: Alphabet> {
    congruence: &'a RightCongruence<A>,
    split: math::OrderedMap<StateIndex, ClassOmegaSample<'a, A>>,
}

impl<'a, A: Alphabet> SplitOmegaSample<'a, A> {
    /// Creates a new object from the given congruence and the split
    pub fn new(
        congruence: &'a RightCongruence<A>,
        split: math::OrderedMap<StateIndex, ClassOmegaSample<'a, A>>,
    ) -> Self {
        Self { congruence, split }
    }

    /// Obtain a reference to the split sample for the given class/index.
    pub fn get(&self, index: StateIndex) -> Option<&ClassOmegaSample<'a, A>> {
        self.split.get(&index)
    }

    /// Obtains an iterator over all classes in the split sample.
    pub fn classes(&self) -> impl Iterator<Item = &'_ Class<A::Symbol>> + '_ {
        self.split.values().map(|sample| &sample.class)
    }

    /// Returns a reference to the underlying congruence.
    pub fn cong(&self) -> &'a RightCongruence<A> {
        self.congruence
    }
}

impl<A: Alphabet> SplitOmegaSample<'_, A> {
    /// Infers a family of right congruences bz first constructing a conflict relation which is then used
    /// as a constraint for the sprout/glerc algorithm.
    pub fn infer_forc(&self) -> FORC<A> {
        let conflict_relations: math::OrderedMap<_, _> = self
            .classes()
            .map(|c| (c.clone(), iteration_consistency_conflicts(self, c.clone())))
            .collect();

        let progress = conflict_relations
            .into_iter()
            .map(|(c, conflicts)| {
                (
                    self.cong().reached_state_index(c).unwrap(),
                    dpainf(
                        conflicts,
                        vec![],
                        // SeparatesIdempotents::new(split_sample.get(&c).expect("This must exist")),
                        false,
                        None,
                    )
                    .expect("Unable to infer a FORC!"),
                )
            })
            .collect_vec();
        FORC::from_iter(self.cong().clone(), progress)
    }
}
