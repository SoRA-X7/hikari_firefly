use std::{
    collections::HashMap,
    env::args,
    process::Stdio,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

use game::tetris::*;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::Command,
    sync::{
        mpsc::{Receiver, Sender},
        Notify,
    },
    task::JoinHandle,
};

const TBP_LOGGING: bool = false;

#[tokio::main]
async fn main() {
    eprintln!("gen_cc");
    let mut game = Game::new();
    let players = game.start(&args().nth(1).expect("usage: gen_cc <exe_path>"));

    let mut update_interval = tokio::time::interval(std::time::Duration::from_millis(16));
    loop {
        update_interval.tick().await;
        game.update();
    }

    tokio::select! {
        _ = players[0] => {
            eprintln!("player 0 exited");
        }
        _ = players[1] => {
            eprintln!("player 1 exited");
        }
    }
}

#[derive(Debug)]
struct Game {
    // players: Vec<Arc<Player>>,
    frame: u64,
    updater: UpdateNotifier,
    damage_buffer: Option<DamageData>,
    damage_queue: std::sync::mpsc::Receiver<DamageData>,
    damage_sender: Arc<std::sync::mpsc::Sender<DamageData>>,
}

impl Game {
    fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            frame: 0,
            updater: UpdateNotifier::new(),
            damage_buffer: None,
            damage_queue: receiver,
            damage_sender: Arc::new(sender),
        }
    }

    fn start(&mut self, exe_path: &str) -> Vec<JoinHandle<()>> {
        let p0 = self.spawn_player(exe_path, 0);
        let p1 = self.spawn_player(exe_path, 1);
        vec![p0, p1]
    }

    fn spawn_player(&mut self, exe_path: &str, id: u32) -> JoinHandle<()> {
        let updater = self.updater.clone();
        let exe_path = exe_path.to_owned();
        let damage_sender = self.damage_sender.clone();
        let p = tokio::spawn(async move {
            let mut p = Player::new(id, &exe_path);
            p.run(updater, damage_sender).await;
        });
        p
    }

    fn update(&mut self) {
        self.frame += 1;
        self.updater.update();

        // do damage calculation
        while let Ok(dmg) = self.damage_queue.try_recv() {
            if dmg.amount == 0 {
                continue;
            }
            eprintln!("damage: {:?} from {}", dmg.amount, dmg.source);
            if let Some(current) = self.damage_buffer.take() {
                if current.source == dmg.source {
                    self.damage_buffer = Some(DamageData {
                        amount: current.amount + dmg.amount,
                        source: current.source,
                        wait: current.wait,
                    });
                } else {
                    // 相殺
                    let amount = dmg.amount as i32 - current.amount as i32;
                    if amount > 0 {
                        self.damage_buffer = Some(DamageData {
                            amount: amount as u32,
                            source: dmg.source,
                            wait: dmg.wait,
                        });
                    } else if amount < 0 {
                        self.damage_buffer = Some(DamageData {
                            amount: -amount as u32,
                            source: current.source,
                            wait: current.wait,
                        });
                    }
                }
            } else {
                self.damage_buffer = Some(dmg);
            }
        }

        if let Some(dmg) = self.damage_buffer.as_mut() {
            if dmg.wait == 0 {
                // apply damage
                eprintln!("apply damage: {:?}", dmg);
                self.damage_buffer = None;
            } else {
                dmg.wait -= 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DamageData {
    amount: u32,
    source: u32,
    wait: u64,
}

#[derive(Debug)]
struct Player {
    id: u32,
    state: GameState<BitBoard>,
    damage_buffer: u32,
    recv: Receiver<tbp::BotMessage>,
    send: Sender<tbp::FrontendMessage>,
    process: tokio::process::Child,
    disconnected: Arc<AtomicBool>,
}

impl Drop for Player {
    fn drop(&mut self) {
        eprintln!("kill");
    }
}

impl Player {
    fn new(id: u32, exe_path: &str) -> Self {
        let mut state = GameState::new();
        for _ in 0..5 {
            state.fulfill_queue();
        }

        let mut process = Command::new(exe_path)
            .kill_on_drop(true)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("spawn failed");

        let out = process.stdout.take().unwrap();
        let mut reader = tokio::io::BufReader::new(out).lines();

        let mut writer = tokio::io::BufWriter::new(process.stdin.take().unwrap());

        eprintln!("spawn ok");

        let (bot_send, recv) = tokio::sync::mpsc::channel(16);

        let disconnected = Arc::new(AtomicBool::new(false));

        let _discon = disconnected.clone();
        tokio::spawn(async move {
            loop {
                if let Some(s) = reader.next_line().await.expect("read line") {
                    if TBP_LOGGING {
                        eprintln!("[RECV/{}] {}", id, s);
                    }
                    let msg = serde_json::from_str(&s).expect("deserialize json");
                    bot_send.send(msg).await.expect("send");
                } else {
                    eprintln!("disconnection");
                    _discon.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
            }
        });

        let (send, mut bot_recv) = tokio::sync::mpsc::channel(16);

        let _discon = disconnected.clone();
        tokio::spawn(async move {
            loop {
                match bot_recv.recv().await {
                    Some(msg) => {
                        let s = serde_json::to_string(&msg).expect("serialize json");
                        if TBP_LOGGING {
                            eprintln!("[SEND/{}] {}", id, s);
                        }
                        writer.write_all(s.as_bytes()).await.expect("write");
                        writer.write_all("\n".as_bytes()).await.expect("write");
                        writer.flush().await.expect("flush");
                    }
                    _ => {
                        eprintln!("disconnection");
                        _discon.store(true, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                }
            }
        });

        eprintln!("ready");

        Self {
            id,
            state,
            recv,
            send,
            damage_buffer: 0,
            process,
            disconnected,
        }
    }

    async fn run(
        &mut self,
        update_notifier: UpdateNotifier,
        damage_sender: Arc<std::sync::mpsc::Sender<DamageData>>,
    ) {
        let msg = self.recv.recv().await;
        if let Some(tbp::BotMessage::Info {
            name,
            version,
            author,
            features,
        }) = msg
        {
            eprintln!(
                "name: {}, version: {}, author: {}, features: {:?}",
                name, version, author, features
            );
        } else {
            return;
        }

        self.send
            .send(tbp::FrontendMessage::Rules {
                randomizer: "seven_bag".to_owned(),
            })
            .await
            .unwrap();

        let msg = self.recv.recv().await;
        if let Some(tbp::BotMessage::Ready) = msg {
            eprintln!("ready");
        } else {
            return;
        }

        // Start and countdown
        self.send
            .send(tbp::FrontendMessage::Start(tbp::Start {
                board: self.state.board.clone().into_colored(CellKind::Gbg),
                queue: self.state.queue.clone().into_iter().collect(),
                hold: self.state.hold,
                combo: (self.state.ren + 1) as u32,
                back_to_back: self.state.b2b,
                randomizer: tbp::Randomizer::SevenBag {
                    bag_state: self.state.bag.0.clone(),
                },
            }))
            .await
            .unwrap();

        update_notifier.wait_for_frames(60).await;

        while self.disconnected.load(std::sync::atomic::Ordering::Relaxed) == false {
            self.run_loop(update_notifier.clone(), damage_sender.clone())
                .await
                .unwrap();
        }
    }

    async fn run_loop(
        &mut self,
        update_notifier: UpdateNotifier,
        damage_sender: Arc<std::sync::mpsc::Sender<DamageData>>,
    ) -> Result<(), BotStopReason> {
        self.state
            .clone()
            .spawn_next()
            .ok_or(BotStopReason::Death)?;

        self.send
            .send(tbp::FrontendMessage::Suggest)
            .await
            .map_err(|_| BotStopReason::Disconnection)?;
        let timer = Instant::now();

        let mut candidates = HashMap::<PieceIdentity, (bool, Move, u16)>::new();
        let gen = {
            self.state
                .legal_moves(true)
                .map_err(|_| BotStopReason::Death)?
        };

        for &(mv, cost) in gen.moves_with_cost().iter() {
            match mv {
                Move::Place(piece) => {
                    candidates.insert(piece.into(), (false, mv, cost));
                }
                Move::Hold => {
                    // calculate possible moves after hold
                    let mut hold_state = self.state.clone();
                    hold_state.advance(mv).ok_or(BotStopReason::Death)?;
                    let hold_gen = hold_state
                        .legal_moves(false)
                        .map_err(|_| BotStopReason::Death)?;
                    for &(aft_mv, aft_cost) in hold_gen.moves_with_cost().iter() {
                        match aft_mv {
                            Move::Place(piece) => {
                                candidates.insert(piece.into(), (true, aft_mv, aft_cost));
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
        // eprintln!("moves found: {:?}", candidates.keys());

        loop {
            match self.recv.recv().await {
                Some(tbp::BotMessage::Suggestion {
                    moves,
                    move_info: _,
                }) => {
                    for (i, piece) in moves.iter().enumerate() {
                        if let Some(&(hold_before, mv, cost)) = candidates.get(&(*piece).into()) {
                            eprintln!(
                                "pick: #{} {:?} at cost {}, elapsed {}us",
                                i,
                                mv,
                                cost,
                                timer.elapsed().as_micros()
                            );

                            if hold_before {
                                update_notifier.wait_for_frames(2).await;
                                self.advance(Move::Hold)
                                    .await
                                    .map_err(|_| BotStopReason::Death)?;
                            }
                            // then
                            let pl = match mv {
                                Move::Place(piece) => {
                                    update_notifier.wait_for_frames(cost as u64).await;
                                    self.advance(Move::Place(piece))
                                        .await
                                        .map_err(|_| BotStopReason::Death)?
                                }
                                _ => unreachable!(),
                            };
                            self.send
                                .send(tbp::FrontendMessage::Play { mv: *piece })
                                .await
                                .map_err(|_| BotStopReason::Disconnection)?;

                            // send damage

                            damage_sender
                                .send(DamageData {
                                    amount: pl.attack(),
                                    source: self.id,
                                    wait: 60,
                                })
                                .unwrap();
                            // placement delay
                            let delay = if pl.lines_cleared > 0 && !pl.is_pc {
                                15
                            } else {
                                0
                            };

                            update_notifier.wait_for_frames(delay).await;
                            return Ok(());
                        }
                    }
                    eprintln!("move {:?} not found", moves);
                    return Err(BotStopReason::Death);
                }
                Some(_) => {
                    return Err(BotStopReason::IllegalMessage);
                }
                _ => return Err(BotStopReason::Disconnection),
            }
        }
    }

    async fn advance(&mut self, mv: Move) -> Result<PlacementResult, ()> {
        eprintln!("advance: {:?}", mv);
        let pl = self.state.advance(mv);

        // Add piece
        let last_piece = self.state.fulfill_queue();
        self.send
            .send(tbp::FrontendMessage::NewPiece { piece: last_piece })
            .await
            .map_err(|_| ())?;

        eprintln!("queue: {:?}, hold: {:?}", self.state.queue, self.state.hold);
        if pl.death {
            Err(())
        } else {
            Ok(pl)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BotStopReason {
    Death,
    IllegalMessage,
    Disconnection,
}

#[derive(Debug, Clone)]
struct UpdateNotifier {
    notify: Arc<Notify>,
}

impl UpdateNotifier {
    fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    async fn wait_for_frames(&self, frames: u64) {
        for _ in 0..frames {
            self.notify.notified().await;
        }
    }

    async fn next_frame(&self) {
        self.notify.notified().await;
    }

    fn update(&self) {
        self.notify.notify_waiters();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PieceIdentity {
    cells: [(i8, i8); 4],
    spin: SpinKind,
}

impl From<PieceState> for PieceIdentity {
    fn from(piece: PieceState) -> Self {
        let mut cells = piece.pos.cells();
        // make sure the cells are always placed in the same order for the canonical representation
        // in order to Eq and Hash work correctly
        cells.sort();
        Self {
            cells,
            spin: piece.spin,
        }
    }
}
