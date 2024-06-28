use std::sync::Arc;

use parking_lot::RwLock;

use crate::graph::Graph;

pub struct Bot {
    graph: Option<Graph>,
}

impl Bot {
    pub fn new() -> Self {
        Self { graph: None }
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
