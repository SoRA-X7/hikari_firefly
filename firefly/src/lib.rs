use std::sync::Arc;

use game::tetris::GameState;
use parking_lot::RwLock;
use search::Graph;

mod mem;
mod search;

pub struct HikariFireflyBot {
    graph: Arc<RwLock<Option<Graph>>>,
}

impl HikariFireflyBot {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        let state = GameState::default(); // todo: reset
        self.graph.write().replace(Graph::new(state));

        for _ in 0..4 {
            let worker = Worker::new(self);
            rayon::spawn(move || {
                worker.work_loop();
            });
        }
    }

    pub fn stop(&self) {
        let mut graph = self.graph.write();
        *graph = None;
    }
}

struct Worker {
    graph: Arc<RwLock<Option<Graph>>>,
}

impl Worker {
    fn new(bot: &HikariFireflyBot) -> Self {
        Self {
            graph: bot.graph.clone(),
        }
    }

    fn work_loop(&self) {
        loop {
            let graph = self.graph.read();
            if let Some(graph) = &*graph {
                graph.work();
            } else {
                println!("Worker {} stopping", rayon::current_thread_index().unwrap());
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let bot = HikariFireflyBot::new();
        bot.start();
        std::thread::sleep(std::time::Duration::from_secs(1));
        bot.stop();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
