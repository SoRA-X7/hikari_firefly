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
        self.bot.read().search()
    }
}
