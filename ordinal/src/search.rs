use std::sync::Arc;

use parking_lot::RwLock;

use crate::graph::Graph;

pub struct Bot {
    graph: Option<Graph>,
}

pub struct BotSync {
    bot: Arc<RwLock<Bot>>,
}

impl BotSync {
    pub fn search(&self) {
        if let Some(graph) = &self.bot.read().graph {
            graph.search();
        }
    }
}
