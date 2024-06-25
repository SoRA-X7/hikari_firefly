use std::sync::atomic::AtomicU32;

use dashmap::DashMap;
use game::tetris::*;
use once_cell::sync::Lazy;

pub struct Graph {
    root_gen: Lazy<Box<Generation>>,
    root_state: GameState<BitBoard>,
}

pub struct Generation {
    nodes: DashMap<StateIndex, Node>,
    next: Lazy<Box<Generation>>,
}

pub struct Node {
    eval: Eval,
    children: Option<Vec<Action>>,
    visits: u32,
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
            if let Some(mv) = node.select_child() {
                state.advance(mv);
            } else {
                node.expand();
            }
        }
    }
}

impl Generation {}

impl Node {
    pub fn select_child(&self) -> Option<&Action> {
        debug_assert!(self.children.is_some());
        let children = self.children.unwrap();
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
        debug_assert!(self.children.is_none());
        state
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
            board: state.board,
            current: state.queue[0],
            hold: state.hold.clone(),
        }
    }
}
