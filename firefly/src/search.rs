use bumpalo_herd::Herd;
use dashmap::DashMap;
use game::tetris::{BitBoard, GameState, Move, SevenBag};
use ouroboros::self_referencing;

pub struct Graph {
    root_gen: Generation,
    root_state: GameState<BitBoard>,
}

#[self_referencing]
pub struct Generation {
    herd: Herd,
    #[borrows(herd)]
    #[not_covariant]
    deduper: DashMap<State, &'this Node<'this>>,
}

pub struct Node<'bump> {
    children: Option<ChildData<'bump>>,
    value: f64,
    acc: f64,
}

pub struct ChildData<'bump>(&'bump [Action]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Action {
    mv: Move,
    reward: f64,
    visits: u32,
}

impl Graph {
    pub fn new(state: GameState<BitBoard>) -> Self {
        Self {
            root_gen: GenerationBuilder {
                herd: Herd::new(),
                deduper_builder: |_| DashMap::new(),
            }
            .build(),
            root_state: state,
        }
    }

    pub fn work(&self) {
        let mut gen = &self.root_gen;
    }
}

impl Generation {
    pub fn get_node(&self, state: &GameState<BitBoard>) -> &Node {
        self.with_deduper(|d| *d.get(&state.clone().into()).unwrap())
    }

    pub fn select(&self, state: &GameState<BitBoard>) -> Action {
        self.get_node(state).select_child()
    }

    pub fn expand(&self, state: &GameState<BitBoard>) {
        self.with(|this| {
            let node = this.deduper.get(&state.clone().into()).unwrap();
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
    pub fn select_child(&self) -> Action {
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
        (*best.unwrap()).clone()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct State {
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
