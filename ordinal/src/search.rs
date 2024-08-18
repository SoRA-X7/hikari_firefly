use std::{collections::VecDeque, convert::Infallible, ops::Deref, sync::Arc};

use futures::prelude::*;
use game::tetris::*;
use parking_lot::RwLock;

use crate::{
    graph::Graph,
    tbp::{BotMessage, FrontendMessage},
};

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

    pub fn add_piece(&mut self, piece: PieceKind) {
        if let Some(graph) = &mut self.graph {
            graph.add_piece(piece);
        }
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

    fn add_piece(&self, piece: PieceKind) {
        if let Some(graph) = &mut self.bot.write().graph {
            graph.add_piece(piece);
        }
    }

    fn stop(&self) {}
}

#[derive(Debug)]
pub struct BotStats {
    pub nodes: Vec<usize>,
}

pub async fn bot_run(
    mut incoming: impl Stream<Item = FrontendMessage> + Unpin,
    mut outgoing: impl Sink<BotMessage, Error = Infallible> + Unpin,
) {
    let bot = BotSync::new();
    bot.init();

    while let Some(msg) = incoming.next().await {
        match msg {
            FrontendMessage::Rules => outgoing.send(BotMessage::Ready).await.unwrap(),
            FrontendMessage::Start(start) => {}
            FrontendMessage::Play { mv } => todo!(),
            FrontendMessage::NewPiece { piece } => bot.add_piece(piece),
            FrontendMessage::Suggest => todo!(),
            FrontendMessage::Stop => bot.stop(),
            FrontendMessage::Quit => break,
            FrontendMessage::Unknown => {}
        }
    }
}
