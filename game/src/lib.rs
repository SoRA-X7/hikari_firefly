use std::{cell::Cell, rc::Rc};

use enumset::{EnumSet, EnumSetType};

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
    fn new(players: Vec<Box<dyn Player>>) -> Self {
        Self { players }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayerKind {
    Tetris,
}

#[derive(Debug)]
pub enum PlayState {
    Tetris {
        state: tetris::GameState<tetris::ColoredBoard>,
    },
}

pub trait Player {
    fn kind(&self) -> PlayerKind;
    fn init(&mut self, game: Rc<Game>, id: u32);
    fn start(&mut self);
    fn update(&mut self);
}

pub struct TetrisPlayer {
    state: tetris::GameState<tetris::ColoredBoard>,
    cooldown: u32,
    grav_fall: f32,
    min_y: i8,
    lock_delay: u32,
    lock_delay_resets: u32,
    current_piece: Option<tetris::PieceState>,
    game: Option<Rc<Game>>,
    id: u32,
    interface: Box<dyn TetrisInterface>,
}

impl TetrisPlayer {
    fn new(interface: Box<dyn TetrisInterface>) -> Self {
        Self {
            state: tetris::GameState::default(),
            cooldown: 0,
            grav_fall: 0.0,
            min_y: 0,
            lock_delay: 0,
            lock_delay_resets: 0,
            current_piece: None,
            game: None,
            id: 0,
            interface,
        }
    }

    fn spawn_next(&mut self) {
        debug_assert!(self.current_piece.is_none());
        if let Some(spawned) = self.state.spawn_next() {
            self.current_piece = Some(spawned);
        } else {
            // Dead
            todo!();
        }
    }

    fn advance(&mut self) {
        let result = self
            .state
            .advance(tetris::Move::Place(self.current_piece.take().unwrap()));
        if result.death {
            // Dead
            todo!();
        }
    }
}

impl Player for TetrisPlayer {
    fn kind(&self) -> PlayerKind {
        PlayerKind::Tetris
    }

    fn init(&mut self, game: Rc<Game>, id: u32) {
        self.game = Some(game.clone());
        self.id = id;
        for _ in 0..5 {
            self.state.fulfill_queue();
        }

        self.interface.init();
    }

    fn start(&mut self) {}

    fn update(&mut self) {
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }

        let controls = self.interface.update(self.cooldown == 0);

        if self.cooldown == 0 {
            let original = self.current_piece.clone().unwrap();

            if self.current_piece.is_none() {
                self.spawn_next();
            }

            if controls.0.contains(Button::Left) {
                if let Some(piece) = self.state.strafe(self.current_piece.unwrap(), (-1, 0)) {
                    self.current_piece = Some(piece);
                }
            }

            if controls.0.contains(Button::Right) {
                if let Some(piece) = self.state.strafe(self.current_piece.unwrap(), (1, 0)) {
                    self.current_piece = Some(piece);
                }
            }

            if controls.0.contains(Button::Cw) {
                if let Some(piece) = self.state.rotate(self.current_piece.unwrap(), true) {
                    self.current_piece = Some(piece);
                }
            }

            if controls.0.contains(Button::Ccw) {
                if let Some(piece) = self.state.rotate(self.current_piece.unwrap(), false) {
                    self.current_piece = Some(piece);
                }
            }

            if controls.0.contains(Button::Hold) {
                if let Some(piece) = self.state.spawn_hold(self.current_piece.unwrap().pos.kind) {
                    self.current_piece = Some(piece)
                } else {
                    // Dead
                }
            }

            if controls.0.contains(Button::Drop) {
                self.grav_fall += 20.0;
            }
            self.grav_fall += 1.0;

            const GRAVITY: f32 = 60.0;
            while self.grav_fall > GRAVITY {
                self.grav_fall -= GRAVITY;
                if let Some(piece) = self.state.strafe(self.current_piece.unwrap(), (0, -1)) {
                    self.current_piece = Some(piece);
                }
            }

            if self.state.is_grounded(&self.current_piece.unwrap()) {
                self.lock_delay += 1;
                if self.lock_delay > 60 {
                    self.advance()
                }
            }

            if original != self.current_piece.unwrap() {
                if self.lock_delay > 0 && self.lock_delay_resets < 15 {
                    self.lock_delay_resets += 1;
                    self.lock_delay = 0;
                }
                if self.min_y > self.current_piece.unwrap().pos.y {
                    self.lock_delay_resets = 0;
                    self.min_y = self.current_piece.unwrap().pos.y
                }
            }
        }
    }
}

pub trait TetrisInterface {
    fn init(&self);
    fn update(&self, can_move: bool) -> Controls;
}
