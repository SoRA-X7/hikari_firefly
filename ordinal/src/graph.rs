use std::sync::atomic::AtomicU32;

use dashmap::DashMap;
use game::tetris::*;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::{
    eval::{Evaluator, SimpleEvaluator},
    movegen::MoveGenerator,
};

pub struct Graph {
    root_gen: Lazy<Box<Generation>>,
    root_state: GameState<BitBoard>,
}

pub struct Generation {
    nodes: DashMap<StateIndex, RwLock<Node>>,
    next: Lazy<Box<Generation>>,
}

pub struct Node {
    eval: Eval,
    children: Option<Vec<Action>>,
    visits: u32,
    dead: bool,
}

pub struct Action {
    mv: Move,
    reward: Eval,
    acc: Eval,
    visits: AtomicU32,
}

type Eval = f32;

impl Graph {
    pub fn search(&self) {
        let mut gen = &**self.root_gen;
        let mut state = self.root_state.clone();
        while let Some(node) = gen.nodes.get(&StateIndex::from_state(&state)) {
            gen = &**gen.next;
            if let Some(act) = node.read().select_child() {
                state.advance(act.mv);
            } else {
                node.write().expand(&state);
            }
        }
    }
}

impl Generation {}

impl Node {
    pub fn select_child(&self) -> Option<&Action> {
        debug_assert!(self.children.is_some());
        let children = self.children.as_ref().unwrap();
        const C: f32 = 1.4;
        let visits_sum_log = (self.visits as f32).ln();
        let selection = children.iter().max_by_key(|a| {
            let v = a.acc
                + C * f32::sqrt(
                    visits_sum_log / a.visits.load(std::sync::atomic::Ordering::Relaxed) as f32,
                );
            (v * 10000.0) as u32
        });
        selection
    }

    pub fn expand(&mut self, state: &GameState<BitBoard>) {
        if self.children.is_some() {
            return;
        }

        if state.queue.is_empty() {
            return;
        }

        if let Ok(generator) = MoveGenerator::generate_for(state, true) {
            let children = generator
                .moves()
                .iter()
                .map(|&mv| {
                    let mut state = state.clone();

                    let reward = if let Move::Place(piece) = mv {
                        let placement = state.place_piece(piece);
                        SimpleEvaluator::reward(&placement)
                    } else {
                        0.0
                    };

                    Action {
                        mv,
                        reward,
                        acc: reward,
                        visits: AtomicU32::new(1),
                    }
                })
                .collect::<Vec<_>>();
            self.children = Some(children);
        } else {
            self.dead = true;
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
struct StateIndex {
    board: BitBoard,
    current: PieceKind,
    hold: Option<PieceKind>,
}

impl StateIndex {
    pub fn from_state(state: &GameState<BitBoard>) -> Self {
        Self {
            board: state.board.clone(),
            current: state.queue[0],
            hold: state.hold.clone(),
        }
    }
}
