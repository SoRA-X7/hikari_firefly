use std::{collections::VecDeque, ops::ControlFlow};

use dashmap::DashMap;
use game::tetris::*;
use once_cell::sync::Lazy;
use rand::{distributions::WeightedIndex, prelude::*};
use smallvec::{smallvec, SmallVec};

use crate::{
    eval::{Accumulator, Evaluator},
    storage::{Index, IndexRange, Rack},
};

#[derive(Debug)]
pub struct Graph<E: Evaluator> {
    root_gen: Box<Generation<E>>,
    root_state: State,
    queue: VecDeque<PieceKind>,
    evaluator: Box<E>,
}

#[derive(Debug)]
pub struct Generation<E: Evaluator> {
    nodes_rack: Rack<Node<E>>,
    actions_rack: Rack<Action<E>>,
    lookup: DashMap<State, Index>,
    parents_lookup: DashMap<Index, SmallVec<[Index; 3]>>,
    next: Lazy<Box<Generation<E>>>,
}

#[derive(Debug, Clone)]
pub struct Node<E: Evaluator> {
    children: Option<ChildData>,
    value: E::Accumulator,
}

#[derive(Debug, Clone)]
pub struct ChildData(IndexRange);

#[derive(Debug)]
pub struct Action<E: Evaluator> {
    node: Index,
    mv: Move,
    current_piece: PieceKind,
    reward: E::TransientReward,
    acc: E::Accumulator,
    visits: u32,
}

impl<E: Evaluator> Graph<E> {
    pub fn new(state: &GameState<BitBoard>, evaluator: Box<E>) -> Self {
        let root_gen = Box::new(Generation::new());
        let root_state = State::new(state);

        // init first node
        let root_node = Node {
            children: None,
            value: evaluator.evaluate_state(state),
        };
        let root_node = root_gen.nodes_rack.alloc(root_node);
        root_gen.parents_lookup.insert(root_node, smallvec![]);
        root_gen.lookup.insert(State::new(state), root_node);

        Self {
            root_gen,
            root_state,
            queue: state.queue.clone(),
            evaluator,
        }
    }

    pub fn work(&self) {
        let mut gen = &*self.root_gen;
        let mut state = self.root_state.clone();
        let mut queue = self.queue.clone();
        let mut gen_history = vec![gen];
        let mut _action_history = vec![];

        loop {
            // Dig down tree until we reach a leaf
            match gen.select(&state) {
                SelectResult::Ok(action) => {
                    _action_history.push(action);
                    state.advance(action.mv, action.current_piece);
                    queue.pop_front().unwrap();
                    gen = &*gen.next;
                    gen_history.push(gen);
                }
                SelectResult::Expand => {
                    if queue.is_empty() {
                        // TODO: speculate
                        return;
                    }
                    gen.expand(&state, queue.pop_front().unwrap(), self.evaluator.as_ref());
                    Self::backprop(gen_history, &state);
                    break;
                }
                SelectResult::Failed => {
                    return;
                }
            }
        }
    }

    pub fn count_nodes(&self) -> usize {
        let mut count = 0;
        let mut gen = &*self.root_gen;
        loop {
            let c = gen.nodes_rack.len();
            if c == 0 {
                break;
            }
            count += c;
            gen = &*gen.next;
        }
        count
    }

    fn backprop(gen_history: Vec<&Generation<E>>, state: &State) {
        puffin::profile_function!();
        let first = gen_history.last().unwrap().find_node_index(state).unwrap();
        let mut to_update = vec![first];

        for &current_gen in gen_history.iter().rev() {
            let mut next_to_update = vec![];
            for index in to_update.iter() {
                current_gen.with_node(*index, |node| {
                    // Update accumulated eval of self
                    let mut children = current_gen
                        .actions_rack
                        .get_range(node.children.as_ref().unwrap().0);
                    children.iter_mut().for_each(|action| {
                        let child_node = current_gen.next.nodes_rack.get(action.node);
                        action.visits += 1;
                        action.acc = child_node.value.accumulate(action.reward);
                    });

                    // best-to-worst sort
                    children.sort_by(|a, b| b.acc.select_score().cmp(&a.acc.select_score()));
                    // let best = children.first().unwrap();
                    // node.acc = best.acc;

                    // Enqueue parents of the current node
                    // TODO: find a better way to lock parents
                    next_to_update.extend(
                        current_gen
                            .parents_lookup
                            .get(index)
                            .as_deref()
                            .unwrap_or(&smallvec![]),
                    );
                });
            }
            to_update = next_to_update;
        }
    }

    pub fn best_plan(&self) -> Plan {
        eprintln!("Queue: {:?}", self.queue);
        let mut gen = &*self.root_gen;
        let mut state = self.root_state.clone();
        let mut moves = vec![];

        for &current_piece in self.queue.iter() {
            let index = gen.find_node_index(&state).unwrap();
            match gen.with_node(index, |node| {
                if let Some(children) = node.children.as_ref() {
                    let best =
                        gen.with_actions(children.0, |actions| actions.first().unwrap().clone());
                    moves.push(best.mv);
                    ControlFlow::Continue(best.mv)
                } else {
                    ControlFlow::Break(())
                }
            }) {
                ControlFlow::Continue(mv) => {
                    state.advance(mv, current_piece);
                    gen = &*gen.next;
                }
                ControlFlow::Break(_) => break,
            }
        }

        Plan { moves, score: 0 }
    }

    pub fn advance(&mut self, mv: Move) -> Result<(), ()> {
        let current_piece = self.queue.pop_front().unwrap();

        let index = self.root_gen.find_node_index(&self.root_state).unwrap();
        self.root_gen.with_node(index, |node| {
            if let Some(children) = node.children.as_ref() {
                self.root_gen.with_actions(children.0, |actions| {
                    if actions.iter().find(|action| action.mv == mv).is_some() {
                        Ok(())
                    } else {
                        Err(())
                    }
                })
            } else {
                Err(())
            }
        })?;
        self.root_state.advance(mv, current_piece);

        let next = std::mem::take(&mut *self.root_gen.next);
        self.root_gen = next;
        Ok(())
    }

    pub fn add_piece(&mut self, piece: PieceKind) {
        self.queue.push_back(piece);

        // TODO: speculate
    }
}

impl<E: Evaluator> Generation<E> {
    pub fn new() -> Self {
        Self {
            nodes_rack: Rack::new(1 << 12),
            actions_rack: Rack::new(1 << 12),
            lookup: DashMap::new(),
            parents_lookup: DashMap::new(),
            next: Lazy::new(|| Box::new(Self::new())),
        }
    }

    pub fn find_node_index(&self, state: &State) -> Option<Index> {
        self.lookup.get(state).map(|x| *x)
    }

    pub fn with_node<R>(&self, index: Index, f: impl FnOnce(&mut Node<E>) -> R) -> R {
        f(&mut self.nodes_rack.get(index))
    }

    pub fn with_actions<R>(&self, range: IndexRange, f: impl FnOnce(&mut [Action<E>]) -> R) -> R {
        f(&mut *self.actions_rack.get_range(range))
    }

    pub fn select(&self, state: &State) -> SelectResult<E> {
        puffin::profile_function!();
        if let Some(index) = self.find_node_index(state) {
            self.with_node(index, |node| {
                if node.children.is_none() {
                    return SelectResult::Expand;
                }

                let children = node.children.as_ref().unwrap();
                let selection = self.with_actions(children.0, |actions| {
                    let min = actions
                        .iter()
                        .map(|action| action.acc.select_score())
                        .fold(i32::MAX, i32::min);
                    let weights = actions
                        .iter()
                        .map(|action| action.acc.select_score() - min + 1)
                        .collect::<Vec<_>>();
                    let dist = WeightedIndex::new(weights).unwrap();
                    let mut rng = thread_rng();
                    let index = dist.sample(&mut rng);
                    let action = actions[index].clone();
                    Some(action)
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

    pub fn expand(&self, state: &State, current_piece: PieceKind, evaluator: &E) {
        puffin::profile_function!();
        let index = self.find_node_index(state).unwrap();

        // Rent shelves to reduce locking
        let mut actions_shelf = self.actions_rack.rent_shelf();
        let mut next_nodes_shelf = self.next.nodes_rack.rent_shelf();
        let next_lookup = &self.next.lookup;
        let next_parent_lookup = &self.next.parents_lookup;

        // Reconstruct GameState
        let game_state = state.reconstruct_with_first_piece(current_piece);

        self.with_node(index, |node| {
            let moves = {
                puffin::profile_scope!("legal_moves");
                game_state.legal_moves(true).unwrap()
            };
            let actions = moves
                .iter()
                .map(|&mv| {
                    let mut game_state = game_state.clone();
                    let placement = game_state.advance(mv);

                    let node_index = next_lookup
                        .entry(State::new(&game_state))
                        .and_modify(|present| {
                            next_parent_lookup.get_mut(present).unwrap().push(index);
                        })
                        .or_insert_with(|| {
                            let eval = evaluator.evaluate_state(&game_state);
                            let node = Node {
                                children: None,
                                value: eval,
                            };
                            let created = next_nodes_shelf.append(node);
                            next_parent_lookup.insert(created, smallvec![index]);
                            created
                        });

                    let reward = evaluator.evaluate_move(mv, placement, &game_state);
                    // let child_node = next_nodes_shelf.get(*node_index.value());
                    let act = Action {
                        node: *node_index.value(),
                        mv,
                        reward,
                        current_piece,
                        acc: E::Accumulator::default(), // will be updated in backprop
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

impl<E: Evaluator> Default for Generation<E> {
    fn default() -> Self {
        Self {
            nodes_rack: Rack::empty(),
            actions_rack: Rack::empty(),
            lookup: DashMap::new(),
            parents_lookup: DashMap::new(),
            next: Lazy::new(|| Box::new(Self::default())),
        }
    }
}

// We need to implement Clone and Copy manually because of the generic parameter
impl<E: Evaluator> Clone for Action<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: Evaluator> Copy for Action<E> {}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    board: BitBoard,
    bag: SevenBag,
    hold: Option<PieceKind>,
    ren: i32,
    b2b: bool,
}

impl State {
    fn new(state: &GameState<BitBoard>) -> Self {
        // Rewind the bag state
        let mut bag = state.bag;
        state.queue.iter().rev().for_each(|&piece| bag.put(piece));

        Self {
            board: state.board.clone(),
            bag,
            hold: state.hold,
            ren: state.ren,
            b2b: state.b2b,
        }
    }

    fn advance(&mut self, mv: Move, mut current_piece: PieceKind) -> Result<PlacementResult, ()> {
        match mv {
            Move::Hold => {
                if self.hold.is_some() {
                    Err(())
                } else {
                    self.hold = Some(current_piece);
                    Ok(PlacementResult::default())
                }
            }
            Move::Place(piece) => {
                if piece.pos.kind != current_piece {
                    let _old = current_piece.clone();
                    current_piece = self.hold.take().expect("hold must not be empty");
                    self.hold = Some(_old);
                }
                debug_assert_eq!(current_piece, piece.pos.kind);

                Ok(self.place_piece(piece))
            }
        }
    }

    fn place_piece(&mut self, piece: PieceState) -> PlacementResult {
        let death = piece.pos.cells().iter().all(|(_, y)| *y >= 20);
        let lines_cleared = self.board.add_piece_and_clear(piece);
        let is_pc = self.board.is_empty();
        let is_b2b = lines_cleared == 4 || (lines_cleared > 0 && piece.spin != SpinKind::None);
        let is_b2b_clear = self.b2b && is_b2b;
        if lines_cleared > 0 {
            self.ren += 1;
            self.b2b = is_b2b
        } else {
            self.ren = -1;
        }
        PlacementResult {
            lines_cleared,
            is_b2b_clear,
            is_pc,
            ren: self.ren,
            spin: piece.spin,
            death,
        }
    }

    fn reconstruct_with_first_piece(&self, current_piece: PieceKind) -> GameState<BitBoard> {
        GameState {
            board: self.board.clone(),
            bag: self.bag,
            queue: VecDeque::from_iter([current_piece]),
            hold: self.hold,
            ren: self.ren,
            b2b: self.b2b,
        }
    }
}

#[derive(Debug)]
pub enum SelectResult<E: Evaluator> {
    /// Node has children, return the best one
    Ok(Action<E>),
    /// Node is a leaf, expand it
    Expand,
    /// Selection function failed
    Failed,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub moves: Vec<Move>,
    pub score: i32,
}
