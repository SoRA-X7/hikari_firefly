use std::cmp::Reverse;

use dashmap::DashMap;
use game::tetris::{BitBoard, GameState, Move, SevenBag};
use once_cell::sync::Lazy;
use smallvec::SmallVec;

use crate::storage::{Index, IndexRange, Rack};

pub struct Graph {
    root_gen: Box<Generation>,
    root_state: GameState<BitBoard>,
}

pub struct Generation {
    nodes_rack: Rack<Node>,
    actions_rack: Rack<Action>,
    parents_lookup: DashMap<State, SmallVec<[Index; 1]>>, // TODO: handle transpositions better
    lookup: DashMap<State, Index>,
    next: Lazy<Box<Generation>>,
}

#[derive(Debug, Clone)]
pub struct Node {
    children: Option<ChildData>,
    value: f64,
    // acc: f64,
}

#[derive(Debug, Clone)]
pub struct ChildData(IndexRange);

#[derive(Debug, Clone, Copy)]
pub struct Action {
    node: Index,
    mv: Move,
    reward: f64,
    acc: f64,
    visits: u32,
}

impl Graph {
    pub fn new(state: &GameState<BitBoard>) -> Self {
        let root_gen = Box::new(Generation::new());
        let root_state = state.clone();
        Self {
            root_gen,
            root_state,
        }
    }

    pub fn work(&self) {
        let mut gen = &*self.root_gen;
        let mut state = self.root_state.clone();
        let mut gen_history = vec![gen];

        loop {
            // Dig down tree until we reach a leaf
            match gen.select(&state) {
                SelectResult::Ok(action) => {
                    state.advance(action.mv);
                    gen = &*gen.next;
                    gen_history.push(gen);
                }
                SelectResult::Expand => {
                    gen.expand(&state);
                    Self::backprop(gen_history, &state);
                    break;
                }
                SelectResult::Failed => {
                    return;
                }
            }
        }
    }

    fn backprop(gen_history: Vec<&Generation>, state: &GameState<BitBoard>) {
        let first = gen_history.last().unwrap().find_node_index(state).unwrap();
        let mut to_update = vec![first];

        for &current_gen in gen_history.iter().rev() {
            let mut next_to_update = vec![];
            for index in to_update.iter() {
                current_gen.with_node(*index, |node| {
                    // Update self accumulated eval
                    let mut children = current_gen
                        .actions_rack
                        .get_range(node.children.as_ref().unwrap().0);
                    children.iter_mut().for_each(|action| {
                        let child_node = current_gen.next.nodes_rack.get(action.node);
                        action.visits += 1;
                        action.acc = child_node.value;
                    });

                    // best-to-worst sort
                    children.sort_by(|a, b| b.eval().partial_cmp(&a.eval()).unwrap());
                    // let best = children.first().unwrap();
                    // node.acc = best.acc;

                    // Enqueue parents of the current node
                    let parents = current_gen
                        .parents_lookup
                        .get(&state.clone().into())
                        .unwrap();
                    // TODO: find a better way to lock parents
                    next_to_update.extend(parents.iter());
                });
            }
            to_update = next_to_update;
        }

        // while let Some(&[parent_gen, current_gen]) = gen_iter.next() {}
    }
}

impl Generation {
    pub fn new() -> Self {
        Self {
            nodes_rack: Rack::new(1 << 12),
            actions_rack: Rack::new(1 << 12),
            lookup: DashMap::new(),
            next: Lazy::new(|| Box::new(Self::new())),
            parents_lookup: DashMap::new(),
        }
    }

    pub fn find_node_index(&self, state: &GameState<BitBoard>) -> Option<Index> {
        self.lookup.get(&state.clone().into()).map(|x| *x)
    }

    pub fn with_node<R>(&self, index: Index, f: impl FnOnce(&mut Node) -> R) -> R {
        f(&mut self.nodes_rack.get(index))
    }

    pub fn with_actions<R>(&self, range: IndexRange, f: impl FnOnce(&mut [Action]) -> R) -> R {
        f(&mut *self.actions_rack.get_range(range))
    }

    pub fn select(&self, state: &GameState<BitBoard>) -> SelectResult {
        if let Some(index) = self.find_node_index(state) {
            self.with_node(index, |node| {
                if node.children.is_none() {
                    return SelectResult::Expand;
                }

                let children = node.children.as_ref().unwrap();
                let selection = self.with_actions(children.0, |actions| {
                    let mut best = None;
                    let mut best_score = f64::NEG_INFINITY;
                    for action in actions {
                        let score = action.reward + (1.0 / (action.visits as f64).sqrt());
                        if score > best_score {
                            best = Some(action);
                            best_score = score;
                        }
                    }
                    Some((*best?).clone())
                });

                match selection {
                    Some(action) => SelectResult::Ok(action),
                    None => SelectResult::Failed,
                }
            })
        } else {
            SelectResult::Failed
        }
    }

    pub fn expand(&self, state: &GameState<BitBoard>) {
        let index = self.find_node_index(state).unwrap();

        // Rent shelves to reduce locking
        let mut actions_shelf = self.actions_rack.rent_shelf();
        let mut next_nodes_shelf = self.next.nodes_rack.rent_shelf();
        let next_lookup = &self.next.lookup;

        self.with_node(index, |node| {
            let actions = state
                .legal_moves(true)
                .unwrap()
                .iter()
                .map(|&mv| {
                    let mut state = state.clone();
                    let _ = state.advance(mv);
                    // TODO: evaluation

                    let node_index = next_lookup.entry(state.into()).or_insert_with(|| {
                        let node = Node {
                            children: None,
                            value: 0.0,
                            // acc: 0.0,
                        };
                        let index = next_nodes_shelf.append(node);
                        index
                    });

                    let act = Action {
                        node: *node_index.value(),
                        mv,
                        reward: 0.0,
                        acc: 0.0,
                        visits: 0,
                    };

                    act
                })
                .collect::<Vec<_>>();

            let child_data = ChildData(actions_shelf.append_vec(actions));
            node.children = Some(child_data);
        });
    }
}

// impl Node {
//     pub fn select_child(&self) -> Option<Action> {
//         debug_assert!(self.children.is_some());
//         let children = self.children.as_ref().unwrap();
//         let total_visits = self.acc;
//         let mut best = None;
//         let mut best_score = f64::NEG_INFINITY;
//         for action in children.0 {
//             let score = action.reward + (1.0 / (action.visits as f64).sqrt());
//             if score > best_score {
//                 best = Some(action);
//                 best_score = score;
//             }
//         }
//         Some((*best?).clone())
//     }

//     pub fn expand_self(&mut self, state: &GameState<BitBoard>, next_gen: &Generation) {
//         debug_assert!(self.children.is_none());
//         let moves = state.legal_moves(true).unwrap();
//         let children = moves.iter().map(|&mv| {
//             let mut state = state.clone();
//             state.advance(mv);
//             Action {
//                 mv,
//                 reward: 0.0,
//                 visits: 0,
//             }
//         });
//     }

//     fn is_leaf(&self) -> bool {
//         self.children.is_none()
//     }
// }

impl Action {
    pub fn eval(&self) -> f64 {
        self.reward + self.acc
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct State {
    board: BitBoard,
    bag: SevenBag,
}

impl From<GameState<BitBoard>> for State {
    fn from(state: GameState<BitBoard>) -> Self {
        Self {
            board: state.board,
            bag: state.bag,
        }
    }
}

#[derive(Debug)]
pub enum SelectResult {
    /// Node has children, return the best one
    Ok(Action),
    /// Node is a leaf, expand it
    Expand,
    /// Selection function failed
    Failed,
}
