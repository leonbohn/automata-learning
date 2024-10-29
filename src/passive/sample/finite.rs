use super::FiniteSample;
use automata::core::alphabet::Alphabet;
use itertools::Itertools;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum FiniteSampleParseError {}

impl<A: Alphabet> FiniteSample<A> {
    /// Create a new sample of finite words from the given alphabet and iterator over annotated words. The sample is given
    /// as an iterator over its symbols. The words are given as an iterator of pairs (word, color).
    pub fn new_finite<I: IntoIterator<Item = A::Symbol>, J: IntoIterator<Item = (I, bool)>>(
        alphabet: A,
        words: J,
    ) -> Self {
        let (positive, negative) = words.into_iter().partition_map(|(w, c)| {
            if c {
                either::Either::Left(w.into_iter().collect())
            } else {
                either::Either::Right(w.into_iter().collect())
            }
        });
        Self {
            alphabet,
            positive,
            negative,
        }
    }

    /// Returns the maximum length of any finite word in the sample. Gives back `0` if no word exists in the sample.
    pub fn max_word_len(&self) -> usize {
        self.words().map(|w| w.len()).max().unwrap_or(0)
    }
}
