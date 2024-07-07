mod lstar;
pub use lstar::*;

pub(crate) mod oracle;
pub use oracle::*;

mod hypothesis;
pub use hypothesis::*;

// mod mealy;
// mod moore;

mod observationtable;
use observationtable::*;

pub mod data {
    pub use super::observationtable::*;
}

#[cfg(test)]
mod tests {
    use automata::{
        automaton::MealyMachine,
        representation::IntoTs,
        transition_system::{Dottable, TSBuilder},
        TransitionSystem,
    };

    use super::{LStar, MealyOracle};

    #[test_log::test]
    fn lstar_mealy() {
        let target = TSBuilder::without_state_colors()
            .with_edges([
                (0, 'a', 0, 0),
                (0, 'b', 1, 1),
                (0, 'c', 2, 2),
                (1, 'a', 0, 2),
                (1, 'b', 1, 1),
                (1, 'c', 2, 2),
                (2, 'a', 2, 2),
            ])
            .into_dts_with_initial(0)
            .into_mealy();
        target.display_rendered();

        let alphabet = target.alphabet().clone();
        let oracle = MealyOracle::new(target, Some(3));
        let learned: MealyMachine = LStar::new(alphabet, oracle).infer();
        learned.display_rendered();
        assert_eq!(learned.size(), 3);
    }
}
