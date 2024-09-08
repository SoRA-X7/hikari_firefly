use std::{
    collections::HashMap,
    env::args,
    process::Stdio,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

use game::tetris::*;
use serde::{ser::SerializeSeq, Serialize};
use smallvec::SmallVec;
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
    let exe_path = args().nth(1).expect("usage: gen_cc <exe_path> <out_dir>");
    let out_dir = args().nth(2).expect("usage: gen_cc <exe_path> <out_dir>");

    let workers = (0..4)
        .map(|i| {
            let exe_path = exe_path.to_owned();
            let out_dir = out_dir.to_owned();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(i * 2)).await;
                loop {
                    eprintln!("worker {} started", i);
                    gen_cc(&exe_path, &out_dir).await;
                }
            })
        })
        .collect::<Vec<_>>();

    for w in workers {
        w.await.expect("worker");
    }
}

async fn gen_cc(exe_path: &str, out_dir: &str) {
    let exe_path = exe_path.to_owned();
    let out_dir = out_dir.to_owned();
    let (out_sender, mut out_receiver) = tokio::sync::mpsc::channel::<Replay>(16);
    let file_writer = tokio::spawn(async move {
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let mut replays = vec![];
        let mut file_id = 0;
        loop {
            if let Some(s) = out_receiver.recv().await {
                // eprintln!("{:?}", s);
                replays.push(s);

                if replays.len() >= 100 {
                    let file_name = format!("{}/gen_cc_{}_{}.bin", out_dir, timestamp, file_id);
                    let mut file = std::fs::File::create(&file_name).expect("create file");
                    rmp_serde::encode::write_named(&mut file, &replays).expect("encode_write");
                    replays.clear();
                    file_id += 1;

                    eprintln!("file written: {}", &file_name);
                }
            } else {
                if !replays.is_empty() {
                    let file_name = format!("{}/gen_cc_{}_{}.bin", out_dir, timestamp, file_id);
                    let mut file = std::fs::File::create(&file_name).expect("create file");
                    rmp_serde::encode::write_named(&mut file, &replays).expect("encode_write");

                    eprintln!("file written: {}", &file_name);
                }
                break;
            }
        }
    });

    let mut game = Game::new();
    game.start(&exe_path, out_sender);

    let mut update_interval = tokio::time::interval(std::time::Duration::from_millis(16));
    let update_worker = tokio::spawn(async move {
        loop {
            update_interval.tick().await;
            game.update().await;
        }
    });

    tokio::spawn(async {
        tokio::select! {
            _ = update_worker => {}
            _ = file_writer => {}
        };
    })
    .await
    .unwrap();
}

#[derive(Debug)]
struct Game {
    players: Vec<PlayerHandle>,
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
            players: Vec::new(),
            frame: 0,
            updater: UpdateNotifier::new(),
            damage_buffer: None,
            damage_queue: receiver,
            damage_sender: Arc::new(sender),
        }
    }

    fn start(&mut self, exe_path: &str, replay_sender: tokio::sync::mpsc::Sender<Replay>) {
        let replay_sender = Arc::new(replay_sender);
        let mut players = Vec::new();
        for i in 0..2 {
            players.push(self.spawn_player(exe_path, i as u32, replay_sender.clone()));
        }
        self.players = players;
    }

    fn spawn_player(
        &mut self,
        exe_path: &str,
        id: u32,
        replay_sender: Arc<tokio::sync::mpsc::Sender<Replay>>,
    ) -> PlayerHandle {
        let updater = self.updater.clone();
        let exe_path = exe_path.to_owned();
        let damage_sender = self.damage_sender.clone();
        let p = PlayerHandle::new(id, &exe_path, updater, damage_sender, replay_sender);
        p
    }

    async fn update(&mut self) {
        self.frame += 1;
        self.updater.update();

        // do damage calculation
        while let Ok(dmg) = self.damage_queue.try_recv() {
            if dmg.amount == 0 {
                continue;
            }
            // eprintln!("damage: {:?} from {}", dmg.amount, dmg.source);
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
                self.players[dmg.source as usize]
                    .garbage_sender
                    .send(dmg.amount)
                    .await
                    .unwrap();
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
struct PlayerHandle {
    garbage_sender: tokio::sync::mpsc::Sender<u32>,
    join_handle: JoinHandle<()>,
}

impl PlayerHandle {
    fn new(
        id: u32,
        exe_path: &str,
        updater: UpdateNotifier,
        damage_sender: Arc<std::sync::mpsc::Sender<DamageData>>,
        replay_sender: Arc<tokio::sync::mpsc::Sender<Replay>>,
    ) -> Self {
        let (garbage_sender, garbage_recv) = tokio::sync::mpsc::channel(16);

        let exe_path = exe_path.to_owned();
        let join_handle = tokio::spawn(async move {
            let mut p = Player::new(id, &exe_path);
            p.run(updater, damage_sender, garbage_recv, replay_sender)
                .await;
        });

        Self {
            garbage_sender,
            join_handle,
        }
    }
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
        mut garbage_recv: tokio::sync::mpsc::Receiver<u32>,
        replay_sender: Arc<tokio::sync::mpsc::Sender<Replay>>,
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
            self.run_loop(
                update_notifier.clone(),
                damage_sender.clone(),
                &mut garbage_recv,
                &replay_sender,
            )
            .await
            .unwrap();
        }
    }

    async fn run_loop(
        &mut self,
        update_notifier: UpdateNotifier,
        damage_sender: Arc<std::sync::mpsc::Sender<DamageData>>,
        garbage_recv: &mut tokio::sync::mpsc::Receiver<u32>,
        replay_sender: &tokio::sync::mpsc::Sender<Replay>,
    ) -> Result<(), BotStopReason> {
        while let Ok(garbage) = garbage_recv.try_recv() {
            self.state.add_garbage(garbage);
            let msg = tbp::FrontendMessage::Start(tbp::Start {
                board: self.state.board.clone().into_colored(CellKind::Gbg),
                queue: self.state.queue.clone().into_iter().collect(),
                hold: self.state.hold,
                combo: (self.state.ren + 1) as u32,
                back_to_back: self.state.b2b,
                randomizer: tbp::Randomizer::SevenBag {
                    bag_state: self.state.bag.0.clone(),
                },
            });
            // eprintln!("apply garbage: {:?}, {:?}", garbage, msg);
            self.send.send(msg).await.unwrap();
            update_notifier.wait_for_frames(10).await;
        }

        let current = self
            .state
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
        let replay = ReplayState {
            board: self.state.board.clone(),
            queue: self.state.queue.iter().copied().collect(),
            current: self.state.queue[0],
            unhold: self.state.hold.map(|x| x).unwrap_or(self.state.queue[1]),
            hold: self.state.hold,
            ren: self.state.ren as i8,
            b2b: self.state.b2b,
            bag: self.state.bag.0.iter().collect(),
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
                Some(tbp::BotMessage::Suggestion { moves, move_info }) => {
                    for (i, piece) in moves.iter().enumerate() {
                        if let Some(&(hold_before, mv, cost)) = candidates.get(&(*piece).into()) {
                            // eprintln!(
                            //     "pick: #{} {:?} at cost {}, elapsed {}us",
                            //     i,
                            //     mv,
                            //     cost,
                            //     timer.elapsed().as_micros()
                            // );

                            let replay = Replay {
                                player_id: self.id,
                                frame: 0,
                                state: replay,
                                action: (*piece).into(),
                            };
                            let replay_send = replay_sender.send(replay);

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
                            replay_send.await.unwrap();
                            return Ok(());
                        }
                    }
                    eprintln!("move {:?} not found {:?}", moves, move_info);
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
        // eprintln!("advance: {:?}", mv);
        let pl = self.state.advance(mv);

        // Add piece
        let last_piece = self.state.fulfill_queue();
        self.send
            .send(tbp::FrontendMessage::NewPiece { piece: last_piece })
            .await
            .map_err(|_| ())?;

        // eprintln!("queue: {:?}, hold: {:?}", self.state.queue, self.state.hold);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
struct Replay {
    player_id: u32,
    frame: u64,
    state: ReplayState,
    action: PieceIdentity,
}

#[derive(Debug, Clone, Serialize)]
struct ReplayState {
    #[serde(serialize_with = "serialize_board")]
    board: BitBoard,
    current: PieceKind,
    unhold: PieceKind,
    queue: SmallVec<[PieceKind; 18]>,
    hold: Option<PieceKind>,
    ren: i8,
    b2b: bool,
    bag: SmallVec<[PieceKind; 7]>,
}

fn serialize_board<S>(board: &BitBoard, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut cols = serializer.serialize_seq(Some(10))?;
    for x in 0..10 {
        let cells: Vec<bool> = (0..64).map(|y| board.occupied((x, y as i8))).collect();
        cols.serialize_element(&cells)?;
    }
    cols.end()
}
