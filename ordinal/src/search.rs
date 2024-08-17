use std::{collections::VecDeque, ops::Deref, sync::Arc};

use game::tetris::*;
use parking_lot::RwLock;

use crate::graph::Graph;

pub struct Bot {
    pub graph: Option<Graph>,
}

impl Bot {
    pub fn new() -> Self {
        Self { graph: None }
    }

    pub fn init(&mut self) {
        let mut state = GameState {
            board: BitBoard::default(),
            hold: None,
            queue: VecDeque::new(),
            bag: SevenBag::default(),
            ren: 0,
            b2b: false,
        };

        state.add_piece(PieceKind::I);
        state.add_piece(PieceKind::T);
        state.add_piece(PieceKind::Z);
        state.add_piece(PieceKind::J);
        state.add_piece(PieceKind::S);

        // println!("{:?}", state);

        self.graph = Some(Graph::new(state));
    }
}

pub struct BotSync {
    bot: Arc<RwLock<Bot>>,
}

impl BotSync {
    pub fn new() -> Self {
        Self {
            bot: Arc::new(RwLock::new(Bot::new())),
        }
    }

    pub fn init(&self) {
        self.bot.write().init();
    }

    pub fn search(&self) {
        if let Some(graph) = &self.bot.read().graph {
            graph.search();
        }
    }

    pub fn work_loop(&self) -> ! {
        loop {
            self.search();
        }
    }

    pub fn stats(&self) -> BotStats {
        BotStats {
            nodes: self.bot.read().graph.as_ref().unwrap().count_nodes(),
        }
    }

    pub fn get(&self) -> impl Deref<Target = Bot> + '_ {
        self.bot.read()
    }
}

#[derive(Debug)]
pub struct BotStats {
    pub nodes: Vec<usize>,
}
