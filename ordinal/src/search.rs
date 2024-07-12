use std::{collections::VecDeque, sync::Arc};

use game::tetris::*;
use parking_lot::RwLock;

use crate::graph::Graph;

pub struct Bot {
    graph: Option<Graph>,
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

        for _ in 0..5 {
            state.fulfill_queue();
        }
        println!("{:?}", state);

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
}
