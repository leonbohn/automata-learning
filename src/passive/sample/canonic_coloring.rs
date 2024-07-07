use automata::prelude::*;

use crate::priority_mapping::ClassifiesIdempotents;

use super::PeriodicOmegaSample;

impl<A: Alphabet> ClassifiesIdempotents<A> for PeriodicOmegaSample<A> {
    fn classify(&self, class: impl FiniteWord<Symbol = <A as Alphabet>::Symbol>) -> Option<bool> {
        self.classify(class.omega_power())
    }
}

#[cfg(test)]
mod tests {
    use automata::prelude::*;

    use crate::passive::dpainf::tests::testing_larger_forc_sample;
    use crate::priority_mapping::AnnotatedCongruence;

    #[test]
    fn classification() {
        let (alphabet, sample) = testing_larger_forc_sample();
        let cong = sample.infer_prefix_congruence().unwrap();
        let split = sample.split(&cong);
        let forc = split.infer_forc();
        let periodic = split.get(0).unwrap().to_periodic_sample();

        let annotated = AnnotatedCongruence::build(forc.prc(0).unwrap(), &periodic);

        let coloring = annotated.canonic_coloring();
        coloring.display_rendered();

        // words we expect prio 1 from
        for w in ["b", "bbabbbb", "aaaaaaabb", "babb", "baabbaabbaabbaa"] {
            assert_eq!(coloring.transform(w), Some(1));
        }
        for w in ["aba", "bbaba", "bbbbbabbaabbbbaaba"] {
            assert_eq!(coloring.transform(w), Some(0));
        }
        assert_eq!(coloring.size(), 13);
    }
}
