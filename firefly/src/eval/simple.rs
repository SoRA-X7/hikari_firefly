use super::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct SimpleEvaluator;

impl Evaluator for SimpleEvaluator {
    type Accumulator = i32;
    type TransientReward = i32;

    fn death_state(&self) -> Self::Accumulator {
        -1000
    }

    fn evaluate_state(&self, state: &GameState<BitBoard>) -> Self::Accumulator {
        let mut score = 0;

        // height
        let max_height = (0..10).map(|x| state.board.height_of(x)).max().unwrap();
        score += max_height as i32 * -10;

        // holes
        let mut holes = 0;
        for x in 0..10 {
            for y in 0..(state.board.height_of(x) as i32 - 1) {
                if !state.board.occupied((x, y as i8)) {
                    holes += 1;
                }
            }
        }
        score += holes * -7;

        score
    }

    fn evaluate_move(
        &self,
        mv: Move,
        placement: PlacementResult,
        state: &GameState<BitBoard>,
    ) -> Self::TransientReward {
        (placement.lines_cleared * 20) as i32
    }
}

impl Accumulator for i32 {
    type Reward = i32;

    fn accumulate(&self, other: Self::Reward) -> Self {
        self + other
    }

    fn select_score(&self) -> i32 {
        *self
    }
}
