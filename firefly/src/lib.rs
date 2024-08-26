use core::fmt;
use std::sync::{atomic::AtomicBool, Arc};

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
    abort: Arc<AtomicBool>,
    config: BotConfig,
}

impl HikariFireflyBot {
    pub fn new(config: BotConfig) -> Self {
        Self {
            graph: Arc::new(RwLock::new(None)),
            abort: Arc::new(AtomicBool::new(true)),
            config,
        }
    }

    pub fn reset(&self, state: Option<GameState<BitBoard>>) {
        eprintln!("reset: {:?}", state);
        let mut graph = self.graph.write();
        *graph = state.map(|s| Graph::new(&s, Box::new(StandardEvaluator::default())));
    }

    pub fn start(&self) {
        self.abort
            .store(false, std::sync::atomic::Ordering::Relaxed);
        for _ in 0..self.config.num_workers {
            let worker = Worker::new(self);
            rayon::spawn(move || {
                worker.work_loop();
            });
        }
    }

    pub fn stop(&self) {
        self.abort.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn suggest(&self) -> Option<Vec<Move>> {
        let graph = self.graph.read();
        if let Some(graph) = &*graph {
            let best = graph.best_plan();
            Some(best.moves)
        } else {
            None
        }
    }

    pub fn pick_move(&self, mv: Move) {
        let mut graph = self.graph.write();
        if let Some(graph) = &mut *graph {
            graph.advance(mv).unwrap();
        }
    }

    pub fn add_piece(&self, piece: PieceKind) {
        let mut graph = self.graph.write();
        if let Some(graph) = &mut *graph {
            graph.add_piece(piece);
        } else {
            eprintln!("No graph available");
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
    abort: Arc<AtomicBool>,
}

impl Worker {
    fn new(bot: &HikariFireflyBot) -> Self {
        Self {
            graph: bot.graph.clone(),
            abort: bot.abort.clone(),
        }
    }

    fn work_loop(&self) {
        eprintln!("Worker {} starting", rayon::current_thread_index().unwrap());

        while !self.abort.load(std::sync::atomic::Ordering::Relaxed) {
            let graph = self.graph.read();
            if let Some(graph) = &*graph {
                graph.work();
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
                // TODO: replace with future
            }
        }

        eprintln!("Worker {} stopping", rayon::current_thread_index().unwrap());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BotConfig {
    pub num_workers: usize,
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
