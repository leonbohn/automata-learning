use std::collections::VecDeque;

use automata::core::alphabet::Alphabet;
use automata::core::math;
use automata::core::word::{OmegaWord, ReducedOmegaWord};
use automata::ts::{Sproutable, StateIndex};
use automata::{Pointed, RightCongruence};
use itertools::Itertools;
use tracing::trace;

pub fn prefix_tree<A: Alphabet, W: Into<ReducedOmegaWord<A::Symbol>>, I: IntoIterator<Item = W>>(
    alphabet: A,
    words: I,
) -> RightCongruence<A, Vec<A::Symbol>> {
    let words: Vec<ReducedOmegaWord<_>> = words.into_iter().map(|word| word.into()).collect_vec();
    debug_assert!(words.iter().all(|word| !word.raw_word().is_empty()));
    fn build_accepting_loop<A: Alphabet>(
        alphabet: &A,
        tree: &mut RightCongruence<A, Vec<A::Symbol>>,
        state: StateIndex,
        access: Vec<A::Symbol>,
        loop_segment: &[A::Symbol],
    ) {
        let mut access = access;
        let mut current = state;
        for symbol in &loop_segment[..loop_segment.len() - 1] {
            access.push(*symbol);
            trace!("adding state {:?}", access);
            let next = tree.add_state(access.clone());
            tree.add_edge((current, alphabet.make_expression(*symbol), next));
            current = next;
        }
        tree.add_edge((
            current,
            alphabet.make_expression(loop_segment[loop_segment.len() - 1]),
            state,
        ));
    }
    let mut tree = RightCongruence::new_with_initial_color(alphabet.clone(), vec![]);
    let root = tree.initial();

    let mut queue = VecDeque::from_iter([(root, vec![], words.to_vec())]);

    while let Some((state, access, words)) = queue.pop_front() {
        debug_assert!(!words.is_empty());
        debug_assert!(words.iter().all(|word| !word.raw_word().is_empty()));
        if words.len() == 1 && words[0].loop_index() == 0 {
            build_accepting_loop(&alphabet, &mut tree, state, access, words[0].cycle());
        } else {
            let mut map: math::Map<_, math::Set<_>> = math::Map::default();
            for mut word in words {
                let sym = word.pop_front();
                debug_assert!(
                    !word.raw_word().is_empty(),
                    "popping front lead to empty representation"
                );

                map.entry(sym).or_default().insert(word);
            }

            for sym in alphabet.universe() {
                if let Some(new_words) = map.swap_remove(&sym) {
                    debug_assert!(!new_words.is_empty());
                    let new_access = access
                        .iter()
                        .cloned()
                        .chain(std::iter::once(sym))
                        .collect_vec();
                    let successor = tree.add_state(new_access.clone());
                    tree.add_edge((state, alphabet.make_expression(sym), successor));
                    queue.push_back((successor, new_access, new_words.into_iter().collect()));
                }
            }
        }
    }

    tree
}

#[cfg(test)]
mod tests {
    use super::prefix_tree;

    #[test]
    #[ignore]
    fn build_prefix_tree() {
        todo!()
        // let words = [upw!("aa"), upw!("aba"), upw!("bbaab"), upw!("bb")];
        // let alphabet = CharAlphabet::from_iter(['a', 'b']);
        // let pta = prefix_tree(alphabet, words);
        // let completed = pta
        //     .erase_state_colors()
        //     .collect_complete_with_initial(Void, Void);
        // let lead_to_sink = ["ba", "bbbbbbbbba", "ababababbbabaababa", "aaaaaaaaaaaaab"];
        // for w in &lead_to_sink {
        //     for v in &lead_to_sink {
        //         assert_eq!(
        //             completed.reached_state_index(w),
        //             completed.reached_state_index(v)
        //         );
        //     }
        // }
    }
}
