use core::fmt::Debug;
use game::tetris::*;

mod simple;
mod standard;

#[allow(unused)]
pub use simple::SimpleEvaluator;
#[allow(unused)]
pub use standard::StandardEvaluator;

pub trait Evaluator: Debug + Sync + Send {
    type TransientReward: Clone + Copy + Debug;
    type Accumulator: Accumulator<Reward = Self::TransientReward> + Clone + Copy + Debug;

    fn evaluate_state(&self, state: &GameState<BitBoard>) -> Self::Accumulator;
    fn evaluate_move(
        &self,
        mv: Move,
        placement: PlacementResult,
        state: &GameState<BitBoard>,
    ) -> Self::TransientReward;
    fn death_state(&self) -> Self::Accumulator;
}

pub trait Accumulator: Debug + Sync + Send + Default {
    type Reward;
    fn accumulate(&self, other: Self::Reward) -> Self;
    fn select_score(&self) -> i32;
}
