use std::{
    hash::{DefaultHasher, Hash},
    ops::DerefMut,
    sync::{atomic::AtomicU32, Arc, Weak},
};

use dashmap::DashMap;
use enumset::EnumSet;
use game::tetris::*;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rand::{seq::SliceRandom, thread_rng, Rng};

use crate::{
    eval::{Evaluator, SimpleEvaluator},
    movegen::MoveGenerator,
};

pub struct Graph {
    root_gen: Lazy<Box<Generation>>,
    root_state: GameState<BitBoard>,
    root_piece: Option<PieceKind>,
}

pub struct Generation {
    nodes: DashMap<StateIndex, NodeSync>,
    next: Lazy<Box<Generation>>,
}

type Eval = f32;

#[derive(Debug)]
pub struct Node {
    acc: Eval,
    field_eval: Eval,
    children: ChildrenData,
    parents: Vec<Weak<RwLock<Node>>>,
    visits: u32,
    dead: bool,
}

#[derive(Debug)]
pub struct ChildrenData {
    speculated: bool,
    candidates: EnumSet<PieceKind>,
    data: [Vec<Action>; 7],
}

#[derive(Clone, Debug)]
pub struct NodeSync(Arc<RwLock<Node>>);

#[derive(Debug)]
pub struct Action {
    mv: Move,
    reward: Eval,
    acc: Eval,
    visits: AtomicU32,
}

impl Graph {
    pub fn new(state: GameState<BitBoard>) -> Self {
        let state_index = StateIndex::new(&state);

        let me = Self {
            root_gen: Lazy::new(|| Box::new(Generation::new())),
            root_state: state,
            root_piece: None,
        };

        me.root_gen.nodes.insert(
            state_index,
            NodeSync::from(Node {
                acc: 0.0,
                field_eval: 0.0,
                children: ChildrenData::new(true),
                parents: vec![],
                visits: 0,
                dead: false,
            }),
        );

        me
    }

    pub fn search(&self) {
        let mut gen = &**self.root_gen;
        let mut state = self.root_state.clone();
        let mut depth = 0;
        let mut moves = vec![];
        while let Some(node) = gen.nodes.get(&StateIndex::new(&state)) {
            // println!("ok");
            let reader = node.0.read();
            if let Some((speculation_resolve, act)) = reader.select_child() {
                if let Some(resolved) = speculation_resolve {
                    state.add_piece(resolved);
                }
                moves.push(act.mv);
                let result = state.advance(act.mv);
                // println!("advance {:?}", state);
                depth += 1;
                gen = &**gen.next;
                // println!("len {}", gen.nodes.len());
                continue;
            } else {
                // println!("expand {:?}", state);
                // println!(
                //     "search_ok depth: {}, moves: {:?}, children: {:?}",
                //     depth,
                //     moves
                //         .iter()
                //         .map(|mv| match mv {
                //             Move::Hold => "(hold)".to_string(),
                //             Move::Place(piece) => piece.pos.kind.to_string(),
                //         })
                //         .collect::<Vec<_>>(),
                //     reader.children
                // );
                drop(reader);
                node.expand(&state, gen);
                return;
            }
        }
        unreachable!();
    }

    pub fn count_nodes(&self) -> Vec<usize> {
        let mut count = vec![];
        let mut gen = &**self.root_gen;
        loop {
            let len = gen.nodes.len();
            if len == 0 {
                break;
            }
            count.push(len);
            gen = &**gen.next;
        }
        count
    }
}

impl Generation {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            next: Lazy::new(|| Box::new(Self::new())),
        }
    }

    pub fn write_node(
        &self,
        state: &GameState<BitBoard>,
        parent: &Arc<RwLock<Node>>,
        node_fn: impl FnOnce() -> Node,
    ) {
        // println!("write {:?}", state);
        let node = self
            .nodes
            .entry(StateIndex::new(state))
            .or_insert_with(|| node_fn().into());
        node.0.write().parents.push(Arc::downgrade(parent));
    }
}

impl Node {
    pub fn select_child(&self) -> Option<(Option<PieceKind>, &Action)> {
        // println!("select {:?}", self.children);

        if self.children.is_empty() {
            return None;
        }

        let speculation_resolve;
        let children = if self.children.is_known() {
            speculation_resolve = None;
            self.children.get_known()
        } else {
            let piece = *self
                .children
                .candidates
                .iter()
                .collect::<Vec<_>>()
                .as_slice()
                .choose(&mut thread_rng())
                .unwrap();
            speculation_resolve = Some(piece);
            self.children.get(piece)
        };
        const C: f32 = 1.4;
        let visits_sum_log = (self.visits as f32).ln();
        let selection = children.iter().max_by_key(|&a| {
            let v = a.acc
                + C * f32::sqrt(
                    visits_sum_log / a.visits.load(std::sync::atomic::Ordering::Relaxed) as f32,
                );
            (v * 10000.0) as u32
        });
        // println!("resolve {:?} {:?}", selection, speculation_resolve);
        selection.and_then(|s| Some((speculation_resolve, s)))
    }

    pub fn back_propagate(&mut self) {
        if self.children.is_known() {
            let acc = Self::process_children(self.field_eval, &mut self.children.get_known_mut());
            self.acc = acc;
        } else {
            let mut accs = vec![];
            for piece in self.children.candidates.iter() {
                let acc =
                    Self::process_children(self.field_eval, &mut self.children.get_mut(piece));
                accs.push(acc);
            }
            self.acc = accs.iter().sum::<f32>() / (accs.len() as f32);
        }

        for parent in &self.parents {
            if let Some(parent) = parent.upgrade() {
                parent.write().back_propagate();
            }
        }
    }

    fn process_children(field_eval: Eval, children: &mut Vec<Action>) -> Eval {
        children.sort_unstable_by(|a, b| a.acc.partial_cmp(&b.acc).unwrap());
        let acc = children[0].acc + field_eval;

        acc
    }
}

impl ChildrenData {
    fn new(speculated: bool) -> Self {
        Self {
            speculated,
            candidates: EnumSet::empty(),
            data: [vec![], vec![], vec![], vec![], vec![], vec![], vec![]],
        }
    }

    fn is_known(&self) -> bool {
        !self.is_speculated()
    }

    fn is_speculated(&self) -> bool {
        self.speculated == true
    }

    fn get_known(&self) -> &Vec<Action> {
        &self.data[self.candidates.as_u8().trailing_zeros() as usize]
    }

    fn get(&self, piece: PieceKind) -> &Vec<Action> {
        &self.data[EnumSet::only(piece).as_u8().trailing_zeros() as usize]
    }

    fn get_known_mut(&mut self) -> &mut Vec<Action> {
        &mut self.data[self.candidates.as_u8().trailing_zeros() as usize]
    }

    fn get_mut(&mut self, piece: PieceKind) -> &mut Vec<Action> {
        &mut self.data[EnumSet::only(piece).as_u8().trailing_zeros() as usize]
    }

    fn set(&mut self, piece: PieceKind, data: Vec<Action>) {
        self.candidates.insert(piece);
        self.data[EnumSet::only(piece).as_u8().trailing_zeros() as usize] = data;
    }

    fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

impl From<Node> for NodeSync {
    fn from(value: Node) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

impl NodeSync {
    pub fn expand(&self, state: &GameState<BitBoard>, gen: &Generation) {
        let mut me = self.0.write();
        if !me.children.is_empty() {
            return;
        }

        let (speculate, candidates) = if state.queue.is_empty() {
            let mut cand = state.bag.0.clone();
            if cand.is_empty() {
                cand = EnumSet::all();
            }
            (true, cand)
        } else {
            (false, EnumSet::only(state.queue[0]))
        };

        me.children.speculated = speculate;

        for piece in candidates.iter() {
            let mut state = state.clone();
            if speculate {
                state.bag.take(piece);
                state.queue.push_back(piece);
            }
            let moves = self.generate_moves(&state, gen);
            if let Some(moves) = moves {
                me.children.set(piece, moves);
            } else {
                me.dead = true;
            }
        }

        me.back_propagate();
    }

    fn generate_moves(&self, state: &GameState<BitBoard>, gen: &Generation) -> Option<Vec<Action>> {
        debug_assert!(!state.queue.is_empty());
        if let Ok(generator) = MoveGenerator::generate_for(state, false) {
            let children = generator
                .moves()
                .iter()
                .map(|&mv| {
                    let mut state = state.clone();

                    let placement = state.advance(mv);

                    let reward = SimpleEvaluator::reward(&placement);

                    let _ = gen.next.write_node(&state, &self.0, || Node {
                        acc: 0.0,
                        field_eval: SimpleEvaluator::eval(&state),
                        children: ChildrenData::new(true),
                        parents: vec![],
                        visits: 0,
                        dead: false,
                    });

                    Action {
                        mv,
                        reward,
                        acc: reward,
                        visits: AtomicU32::new(1),
                    }
                })
                .collect::<Vec<_>>();
            Some(children)
        } else {
            None
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
struct StateIndex {
    board: BitBoard,
    hold: Option<PieceKind>,
    speculation_pieces: EnumSet<PieceKind>,
}

impl StateIndex {
    pub fn new(state: &GameState<BitBoard>) -> Self {
        Self {
            board: state.board.clone(),
            hold: state.hold.clone(),
            speculation_pieces: state.bag.0,
        }
    }
}
