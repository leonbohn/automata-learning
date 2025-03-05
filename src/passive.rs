use automata::automaton::{DBA, DFA, DPA, MealyMachine};
use automata::core::alphabet::{Alphabet, CharAlphabet};
use automata::representation::{CollectTs, IntoTs};
use automata::ts::Deterministic;
use automata::ts::operations::Product;
use automata::{
    TransitionSystem,
    ts::{
        IndexedAlphabet,
        operations::{DefaultIfMissing, MapStateColor},
    },
};
use owo_colors::OwoColorize;
use tracing::{debug, trace};

/// Contains definitions for samples, which are collections of positive and
/// negative example words.
#[macro_use]
pub mod sample;
pub use sample::{ClassOmegaSample, PeriodicOmegaSample, SetSample, SplitOmegaSample};

use crate::{
    AnnotatedCongruence,
    active::{CompletingMealyOracle, LStar},
    passive::fwpm::FWPM,
    prefixtree::prefix_tree,
};

use self::precise::PreciseDPA;

pub use self::sample::{FiniteSample, OmegaSample};

/// Module containing the implementations of the sprout/glerc algorithm.
pub mod dpainf;

/// Deals with families of weak priority mappings.
pub mod fwpm;

/// Defines the precise DPA.
pub mod precise;

/// Module containing the implementation of the sprout algorithm.
pub mod sprout;

/// Executes the RPNI algorithm on the given sample. This returns a DFA that is
/// composed of a right congruence as well as an acceptance condition, which marks
/// a classes as accepting if it is reached by a positive sample word.
pub fn dfa_rpni<A: Alphabet>(sample: &FiniteSample<A>) -> DFA<A> {
    let cong = dpainf::dpainf(sample, vec![], true, None).unwrap();
    let accepting = sample
        .positive_words()
        .map(|w| {
            (
                cong.reached_state_index(w)
                    .expect("Every positive word must induce a successful run!"),
                true,
            )
        })
        .collect();
    cong.with_state_color(DefaultIfMissing::new(accepting, false))
        .into_dfa()
}

/// Executes a variant of the RPNI algorithm for omega-words, producing a DBA.
pub fn dba_rpni<A: Alphabet>(sample: &OmegaSample<A>) -> DBA<A> {
    todo!()
}

/// Takes a reference to an [`OmegaSample`], which classifies infinite words over the alphabet `A`
/// with boolean values and infers a [`PreciseDPA`] from it. The steps for this are roughly
/// - infer the leading prefix (aka Myhill/Nerode) congruence
/// - produce a [`SplitOmegaSample`] such that each suffix of a sample word which "starts" in a class `c`
///   gets put into the respective component of the sample for class `c`
/// - uses the glerc/sprout algorithm to infer a family of right congruences (FORC)
/// - color the FORC to obtain a canonical coloring for each class; these are then combined to an FWPM using
///   the leading congruence infered before
/// - build the precise DPA from the FWPM
pub fn infer_precise_dpa<A: Alphabet>(
    sample: &OmegaSample<A>,
) -> PreciseDPA<A, { precise::PRECISE_DPA_COLORS }> {
    let cong = sample.infer_prefix_congruence().unwrap();
    let split = sample.split(&cong);

    let forc = split.infer_forc();
    trace!("{}\n{:?}", "INFERRED FORC".bold(), forc);

    let mut fwpm = FWPM::empty(cong.clone());
    for (class, idx) in cong.classes() {
        let periodic_sample = split.get(idx).expect("Must exist!").to_periodic_sample();
        let annotated_prc = AnnotatedCongruence::build(&forc[idx], &periodic_sample);
        trace!(
            "{} for class {:?}\t{:?}",
            "ANNOTATED CONGRUENCE".bold().blue(),
            class,
            annotated_prc
        );
        let coloring = annotated_prc.canonic_coloring();
        trace!("{}{:?}", "inferred ".green(), coloring);
        fwpm.insert_pm(idx, coloring);
    }
    trace!("Calculated the FWPM\n{:?}", fwpm);
    fwpm.into_precise_dpa()
}

/// Similar to [`dba_rpni`], but produces a DPA instead.
pub fn dpa_rpni(sample: &OmegaSample<CharAlphabet>) -> DPA {
    let precise = infer_precise_dpa(sample);
    let pta = sample.prefix_tree().erase_state_colors();

    let prod = precise
        .ts_product(pta)
        .map_edge_colors(|(c, _)| c)
        .erase_state_colors();
    let (completed, initial) = prod.trim_collect_pointed();

    //now we use the completed thing to learn a MealyMachine from which we can then build the DPA
    let mm = completed.with_initial(initial).collect_mealy();

    let alphabet = mm.alphabet().clone();
    let oracle = CompletingMealyOracle::new(mm, 0);

    let start = std::time::Instant::now();
    let learned: MealyMachine = LStar::new(alphabet, oracle).infer();
    debug!(
        "Learning representation of DPA with LStar took {}ms",
        start.elapsed().as_millis()
    );
    learned.collect_dpa()
}

fn characterize_dpa(dpa: DPA) -> OmegaSample {
    let cong = dpa.prefix_congruence();

    todo!()
}

#[cfg(test)]
mod tests {
    use crate::passive::dpa_rpni;
    use automata::TransitionSystem;
    use automata::core::alphabet::CharAlphabet;
    use automata::core::upw;
    use automata::representation::CollectTs;
    use tracing::info;

    use super::{OmegaSample, sample};

    #[test]
    fn infer_precise_dpa_with_al_inf_aa() {
        let alphabet = CharAlphabet::of_size(3);
        let sample = OmegaSample::new_omega_from_pos_neg(
            alphabet,
            [
                upw!("a"),
                upw!("aab"),
                upw!("aba"),
                upw!("aaab"),
                upw!("bbaa"),
                upw!("aca"),
                upw!("caa"),
                upw!("aac"),
                upw!("abca"),
                upw!("baac"),
            ],
            [
                upw!("c"),
                upw!("b"),
                upw!("bc"),
                upw!("abc"),
                upw!("acb"),
                upw!("acc"),
                upw!("abb"),
                upw!("cba"),
                upw!("ac"),
                upw!("ba"),
            ],
        );

        let t = std::time::Instant::now();
        let dpa = super::infer_precise_dpa(&sample).collect_dpa();
        let full_duration = t.elapsed().as_millis();
        info!(
            "full construction of precise DPA size {} took {full_duration}ms",
            dpa.size()
        );

        let expected = [
            (upw!("cabaca"), false),
            (upw!("a"), true),
            (upw!("baa"), true),
            (upw!("baacbacbac"), true),
        ];
        for (w, c) in &expected {
            let b = dpa.accepts(w);
            assert_eq!(b, *c, "{:?} is classified {b}, expected {c}", w);
        }

        let t = std::time::Instant::now();
        let dpa_mm = dpa_rpni(&sample);

        let paper_duration = t.elapsed().as_millis();
        info!(
            "Full construction took {full_duration}ms, paper construction took {paper_duration}ms"
        );
        assert!(dpa_mm.size() <= dpa.size());

        for (w, c) in expected {
            let b = dpa_mm.accepts(&w);
            assert_eq!(b, c, "{:?} is classified {b}, expected {c}", w);
        }
    }
}
