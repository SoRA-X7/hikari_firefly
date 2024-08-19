use std::cell::Cell;

use bumpalo_herd::Herd;
use ouroboros::self_referencing;
use rand::Rng;

use dashmap::{
    mapref::{
        entry::{self, Entry},
        one::RefMut,
    },
    DashMap,
};
use game::tetris::*;
use once_cell::sync::OnceCell;

use crate::{
    mem::{HerdPool, RentedHerd},
    movegen::MoveGenerator,
};

pub struct Graph {
    root_gen: Box<Generation>,
    root_state: GameState<BitBoard>,
}

impl Graph {
    pub fn search_work(&self) {
        let mut gen = self.root_gen;
        let mut state = self.root_state.clone();
        let mut use_hold = true;
        loop {}
    }
}

#[self_referencing]
pub struct Generation {
    herd: Herd,
    #[borrows(herd)]
    #[not_covariant]
    deduper: DashMap<State, &'this Node>,
    next: OnceCell<Box<Generation>>,
}

#[derive(Debug, Clone)]
pub struct Children {
    pub data: Vec<Action>,
}

impl Children {
    pub fn build_with(
        moves: Vec<Move>,
        next_gen: &Generation,
        state: &GameState<BitBoard>,
    ) -> Self {
        let mut actions = Vec::new();
        for (index, &mv) in moves.iter().enumerate() {
            let mut state = state.clone();
            state.advance(mv);
            let node = next_gen.upsert_node(state.into()).index();
            let reward = 0.0;
            actions.push(Action {
                mv,
                node,
                reward,
                visits: 0,
            });
        }
        Children { data: actions }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub mv: Move,
    pub node: usize,
    pub reward: f64,
    pub visits: u32,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub children: Option<Children>,
    pub visits: u32,
    pub value: f64,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct State {
    pub board: BitBoard,
    pub bag: SevenBag,
}

impl From<GameState<BitBoard>> for State {
    fn from(state: GameState<BitBoard>) -> Self {
        Self {
            board: state.board,
            bag: state.bag,
        }
    }
}

impl Generation {
    pub fn build() -> Self {
        GenerationBuilder {
            herd: Herd::default(),
            deduper_builder: |herd| DashMap::new(),
            next: OnceCell::new(),
        }
        .build()
    }

    pub fn 

    // pub fn node<'a>(&'a self, state: State, make: impl FnOnce() -> Node) -> &'a Node {
    //     self.with(|this| {
    //         let bump: bumpalo_herd::Member<'a> = this.herd.get();
    //         this.deduper
    //             .entry(state)
    //             .or_insert_with(move || bump.alloc(make()))
    //             .value_mut()
    //     })
    // }
}

impl Node {
    pub fn new() -> Self {
        Self {
            children: None,
            visits: 0,
            value: 0.0,
        }
    }

    pub fn select(&mut self) -> Option<&mut Action> {
        debug_assert!(self.children.is_some());
        let children = self.children.as_mut().unwrap();

        let total_visits: u32 = children.data.iter().map(|action| action.visits).sum();
        let mut rng = rand::thread_rng();
        let mut weighted_actions: Vec<(f64, &mut Action)> = children
            .data
            .iter_mut()
            .map(|action| {
                let weight = action.visits as f64 / total_visits as f64;
                (weight, action)
            })
            .collect();
        weighted_actions.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let random_value: f64 = rng.gen();
        let mut cumulative_weight = 0.0;
        for (weight, action) in weighted_actions {
            cumulative_weight += weight;
            if cumulative_weight >= random_value {
                action.visits += 1;
                return Some(action);
            }
        }
        None
    }

    pub fn expand(&mut self, state: &GameState<BitBoard>, use_hold: bool, next_gen: &Generation) {
        debug_assert!(self.children.is_none());

        let move_gen = MoveGenerator::generate_for(state, use_hold);
        if let Ok(move_gen) = move_gen {
            let moves = move_gen.moves();

            let children = Children::build_with(moves, next_gen, state);

            self.children = Some(children);
        }
    }
}
