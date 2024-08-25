use core::fmt;
use std::sync::Arc;

use eval::standard::StandardEvaluator;
use game::tetris::*;
use parking_lot::RwLock;
use search::Graph;

mod eval;
mod mem;
mod search;
mod storage;

type E = StandardEvaluator;

#[derive(Debug)]
pub struct HikariFireflyBot {
    graph: Arc<RwLock<Option<Graph<E>>>>,
}

impl HikariFireflyBot {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        let mut state = GameState::new();
        for _ in 0..12 {
            state.fulfill_queue();
        }
        println!("Initial state: {:?}", state);

        self.graph
            .write()
            .replace(Graph::new(&state, Box::new(StandardEvaluator::default())));

        for _ in 0..3 {
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

    pub fn suggest(&self) -> Option<Vec<Move>> {
        let graph = self.graph.read();
        if let Some(graph) = &*graph {
            let best = graph.best_plan();
            println!("Best: {:?}", best);
            Some(best.moves)
        } else {
            println!("No graph available");
            None
        }
    }

    pub fn pick_move(&self, mv: Move) {
        let mut graph = self.graph.write();
        if let Some(graph) = &mut *graph {
            graph.advance(mv);
        } else {
            println!("No graph available");
        }
    }

    pub fn stats(&self) -> usize {
        let graph = self.graph.read();
        if let Some(graph) = &*graph {
            graph.count_nodes()
        } else {
            0
        }
    }
}

#[derive(Debug)]
struct Worker {
    graph: Arc<RwLock<Option<Graph<E>>>>,
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

#[derive(Debug, Clone)]
pub struct Stats {
    pub nodes: Vec<usize>,
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stats {{ nodes: {:?} }}", self.nodes)
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
