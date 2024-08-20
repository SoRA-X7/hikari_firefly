use bumpalo_herd::Herd;
use dashmap::DashMap;
use game::tetris::{BitBoard, GameState, Move, SevenBag};
use once_cell::sync::Lazy;
use ouroboros::self_referencing;

pub struct Graph {
    root_gen: Box<Generation>,
    root_state: GameState<BitBoard>,
}

#[self_referencing]
pub struct Generation {
    // Nodes and child data are all stored in this
    herd: Herd,
    #[borrows(herd)]
    #[not_covariant]
    lookup: DashMap<State, &'this Node<'this>>,
    next: Lazy<Box<Generation>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Node<'bump> {
    children: Option<ChildData<'bump>>,
    value: f64,
    acc: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ChildData<'bump>(&'bump [Action]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Action {
    mv: Move,
    reward: f64,
    visits: u32,
}

impl Graph {
    pub fn new(state: GameState<BitBoard>) -> Self {
        let this = Self {
            root_gen: Box::new(Generation::build()),
            root_state: state,
        };
        this.root_gen.write_node(state);
        this
    }

    pub fn work(&self) {
        let mut gen = &self.root_gen;
        let mut state = self.root_state.clone();
        loop {
            // Dig down tree until we reach a leaf
            match gen.select(&state) {
                SelectResult::Ok(action) => {
                    state.advance(action.mv);
                    gen = gen.borrow_next();
                }
                SelectResult::Expand => {
                    gen.expand(&state);
                    break;
                }
                SelectResult::Failed => {
                    return;
                }
            }
        }
    }
}

impl Generation {
    pub fn build() -> Self {
        GenerationBuilder {
            herd: Herd::new(),
            lookup_builder: |_| DashMap::new(),
            next: Lazy::new(|| Box::new(Generation::build())),
        }
        .build()
    }

    pub fn get_node(&self, state: &GameState<BitBoard>) -> &Node {
        self.with_lookup(|d| *d.get(&state.clone().into()).unwrap())
    }

    pub fn write_node(&self, state: &GameState<BitBoard>, node: &Node<'_>) {
        self.with(|this| {
            let l = this.lookup;
            let s = state.clone().into();
            l.insert(s, node);
        });
    }

    pub fn select(&self, state: &GameState<BitBoard>) -> SelectResult {
        let node = self.get_node(state);
        if node.is_leaf() {
            SelectResult::Expand
        } else {
            match node.select_child() {
                Some(action) => SelectResult::Ok(action),
                None => SelectResult::Failed,
            }
        }
    }

    pub fn expand(&self, state: &GameState<BitBoard>) {
        self.with(|this| {
            let node = this.lookup.get(&state.clone().into()).unwrap();
            let mut children = Vec::new();
            for mv in state.legal_moves(true).unwrap() {
                children.push(Action {
                    mv,
                    reward: 0.0,
                    visits: 0,
                });
            }
            ChildData(this.herd.get().alloc_slice_copy(&children))
        });
    }
}

impl Node<'_> {
    pub fn select_child(&self) -> Option<Action> {
        debug_assert!(self.children.is_some());
        let children = self.children.as_ref().unwrap();
        let total_visits = self.acc;
        let mut best = None;
        let mut best_score = f64::NEG_INFINITY;
        for action in children.0 {
            let score = action.reward + (1.0 / (action.visits as f64).sqrt());
            if score > best_score {
                best = Some(action);
                best_score = score;
            }
        }
        Some((*best?).clone())
    }

    pub fn expand_self(&mut self, state: &GameState<BitBoard>, next_gen: &Generation) {
        debug_assert!(self.children.is_none());
        let moves = state.legal_moves(true).unwrap();
        let children = moves.iter().map(|&mv| {
            let mut state = state.clone();
            state.advance(mv);
            Action {
                mv,
                reward: 0.0,
                visits: 0,
            }
        });
    }

    fn is_leaf(&self) -> bool {
        self.children.is_none()
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
