use automata::prelude::*;

use crate::priority_mapping::ClassifiesIdempotents;

use super::PeriodicOmegaSample;

impl<A: Alphabet> ClassifiesIdempotents<A> for PeriodicOmegaSample<A> {
    fn classify(&self, class: impl FiniteWord<<A as Alphabet>::Symbol>) -> Option<bool> {
        self.classify(class.omega_power())
    }
}

#[cfg(test)]
mod tests {
    use automata::{transition_system::Dottable, RightCongruence, TransitionSystem};

    use crate::passive::dpainf::tests::testing_larger_forc_sample;
    use crate::priority_mapping::AnnotatedCongruence;

    #[test]
    #[ignore]
    fn classification() {
        let (alphabet, sample) = testing_larger_forc_sample();
        let cong = sample.infer_prefix_congruence();
        let split = sample.split(&cong);
        let forc = split.infer_forc();
        let periodic = split.get(0).unwrap().to_periodic_sample();

        let annotated = AnnotatedCongruence::build(forc.prc(0).unwrap(), &periodic);
        println!("{:?}", annotated);

        let coloring = annotated.canonic_coloring();
        // coloring
        //     .collect_with_initial::<RightCongruence<_, usize, _>>()
        //     .display_rendered();
    }
}
