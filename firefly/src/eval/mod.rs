use game::tetris::*;

mod classic;

trait Evaluator {
    fn evaluate_state(&self, state: &GameState<BitBoard>) -> f64;
    fn evaluate_move(&self, mv: Move, state: &GameState<BitBoard>) -> f64;
}
