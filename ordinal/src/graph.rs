use std::{
    ops::DerefMut,
    sync::{atomic::AtomicU32, Arc, Weak},
};

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
    nodes: DashMap<StateIndex, NodeSync>,
    next: Lazy<Box<Generation>>,
}

type Eval = f32;

#[derive(Debug)]
pub struct Node {
    acc: Eval,
    field_eval: Eval,
    children: Option<Vec<Action>>,
    parents: Vec<Weak<RwLock<Node>>>,
    visits: u32,
    dead: bool,
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
        let state_index = StateIndex::from_state(&state);

        let me = Self {
            root_gen: Lazy::new(|| Box::new(Generation::new())),
            root_state: state,
        };

        me.root_gen.nodes.insert(
            state_index,
            NodeSync::from(Node {
                acc: 0.0,
                field_eval: 0.0,
                children: None,
                parents: vec![],
                visits: 0,
                dead: false,
            }),
        );

        me
    }

    pub fn search(&self) {
        println!("start search");
        let mut gen = &**self.root_gen;
        let mut state = self.root_state.clone();
        let mut depth = 0;
        while let Some(node) = gen.nodes.get(&StateIndex::from_state(&state)) {
            println!("ok");
            let reader = node.0.read();
            if let Some(act) = reader.select_child() {
                let result = state.advance(act.mv);
                println!("advance {:?}", state);
                depth += 1;
                gen = &**gen.next;
                println!("len {}", gen.nodes.len());
                continue;
            } else {
                println!("expand");
                drop(reader);
                node.expand(&state, gen);
                println!("{}", node.0.read().children.as_ref().unwrap().len());
                node.0.write().back_propagate();
                println!("search_ok depth: {}", depth);
                return;
            }
        }
        unreachable!();
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
        println!("write {:?}", state);
        let node = self
            .nodes
            .entry(StateIndex::from_state(state))
            .or_insert_with(|| node_fn().into());
        node.0.write().parents.push(Arc::downgrade(parent));
    }
}

impl Node {
    pub fn select_child(&self) -> Option<&Action> {
        if let Some(children) = &self.children {
            const C: f32 = 1.4;
            let visits_sum_log = (self.visits as f32).ln();
            let selection = children.iter().max_by_key(|&a| {
                let v = a.acc
                    + C * f32::sqrt(
                        visits_sum_log / a.visits.load(std::sync::atomic::Ordering::Relaxed) as f32,
                    );
                (v * 10000.0) as u32
            });
            selection
        } else {
            None
        }
    }

    pub fn back_propagate(&mut self) {
        if let Some(ref mut children) = &mut self.children {
            children.sort_unstable_by(|a, b| a.acc.partial_cmp(&b.acc).unwrap());
            self.acc = children[0].acc + self.field_eval;

            for parent in &self.parents {
                if let Some(parent) = parent.upgrade() {
                    parent.write().back_propagate();
                }
            }
        }
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
        if me.children.is_some() {
            return;
        }

        if state.queue.is_empty() {
            return;
        }
        println!("gen");

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
                        children: None,
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
            // println!("{:?}", children);
            me.children = Some(children);
        } else {
            me.dead = true;
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
