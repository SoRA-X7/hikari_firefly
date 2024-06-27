use game::tetris::*;

// pub trait Evaluation: Add + PartialOrd + Sized {}

// impl Evaluation for f64 {}

pub trait Evaluator {
    fn eval(state: &GameState<BitBoard>) -> f32;
    fn reward(placement: &PlacementResult) -> f32;
}

pub struct SimpleEvaluator {
    max_height: f32,
}

impl Evaluator for SimpleEvaluator {
    fn eval(state: &GameState<BitBoard>) -> f32 {
        let mut eval = 0f32;

        let max_height = state
            .board
            .cols
            .iter()
            .map(|col| u64::BITS - col.leading_zeros())
            .max();

        return eval;
    }

    fn reward(placement: &PlacementResult) -> f32 {
        let mut reward = 0f32;

        if placement.is_pc {
            reward += 1000.0
        }

        reward += match placement.spin {
            SpinKind::Full => match placement.lines_cleared {
                3 => 900.0,
                2 => 600.0,
                1 => 200.0,
                _ => 0.0,
            },
            SpinKind::Mini => 50.0,
            SpinKind::None => match placement.lines_cleared {
                4 => 700.0,
                3 => -50.0,
                2 => -300.0,
                1 => -350.0,
                _ => 0.0,
            },
        };

        reward
    }
}
