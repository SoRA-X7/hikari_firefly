use std::rc::Rc;

use crate::{tetris::*, *};

use super::*;

pub struct TetrisPlayer {
    state: GameState<ColoredBoard>,
    cooldown: u32,
    grav_fall: f32,
    min_y: i8,
    lock_delay: u32,
    lock_delay_resets: u32,
    current_piece: Option<PieceState>,
    game: Option<Rc<Game>>,
    id: u32,
    interface: Box<dyn TetrisInterface>,
}

impl TetrisPlayer {
    pub fn new(interface: Box<dyn TetrisInterface>) -> Self {
        let mut state = GameState::default();
        state.fulfill_queue();

        Self {
            state,
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

    pub fn spawn_next(&mut self) {
        debug_assert!(self.current_piece.is_none());
        if let Some(spawned) = self.state.spawn_next() {
            self.current_piece = Some(spawned);
        } else {
            // Dead
            todo!();
        }
    }

    pub fn advance(&mut self) {
        let result = self
            .state
            .advance(Move::Place(self.current_piece.take().unwrap()));
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
            if self.current_piece.is_none() {
                self.spawn_next();
                return;
            }

            let original = self.current_piece.clone().unwrap();

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

pub struct NopInterface;
impl TetrisInterface for NopInterface {
    fn init(&self) {}
    fn update(&self, can_move: bool) -> Controls {
        Controls(EnumSet::empty())
    }
}
