use std::rc::Rc;

use crate::Game;

pub mod tetris;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayerKind {
    Tetris,
}

#[derive(Debug)]
pub enum PlayState {
    Tetris {
        state: crate::tetris::GameState<crate::tetris::ColoredBoard>,
    },
}

pub trait Player {
    fn kind(&self) -> PlayerKind;
    fn init(&mut self, game: Rc<Game>, id: u32);
    fn start(&mut self);
    fn update(&mut self);
}
