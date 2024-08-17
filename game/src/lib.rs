use std::{cell::Cell, rc::Rc};

use enumset::{EnumSet, EnumSetType};
use player::Player;

pub mod player;
pub mod tetris;

#[derive(EnumSetType)]
pub enum Button {
    Left,
    Right,
    Drop,
    Lock,
    Cw,
    Ccw,
    Hold,
}

pub struct Controls(EnumSet<Button>);

pub struct Game {
    players: Vec<Box<dyn Player>>,
}

impl Game {
    pub const fn new(players: Vec<Box<dyn Player>>) -> Self {
        Self { players }
    }

    pub fn update(&mut self) {
        self.players.iter_mut().for_each(|p| p.update());
    }
}
