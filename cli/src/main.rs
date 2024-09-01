use firefly::{BotConfig, HikariFireflyBot};
use game::tetris::{tbp::*, GameState, Move, SevenBag};
use tokio::io::AsyncBufReadExt;

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut bot = None;
            let mut last_reply_moves = vec![];

            write_message(BotMessage::Info {
                name: "Hikari".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                author: "SoRA-X7".to_owned(),
                features: vec!["randomizer".to_owned()],
            });

            let mut reader = tokio::io::BufReader::new(tokio::io::stdin()).lines();
            while let Some(line) = reader.next_line().await.unwrap() {
                if let Ok(result) = serde_json::from_str::<FrontendMessage>(&line) {
                    match result {
                        FrontendMessage::Rules => {
                            bot = Some(HikariFireflyBot::new(BotConfig { num_workers: 1 }));
                            write_message(BotMessage::Ready);
                        }
                        FrontendMessage::Start(start) => {
                            let Some(bot) = &bot else {
                                break;
                            };
                            bot.reset(Some(GameState {
                                board: start.board.into(),
                                queue: start.queue.into(),
                                hold: start.hold,
                                ren: start.combo as i32 - 1,
                                b2b: start.back_to_back,
                                bag: match start.randomizer {
                                    Randomizer::SevenBag { bag_state } => SevenBag(bag_state),
                                    _ => unimplemented!(),
                                },
                            }));
                            bot.start();
                        }
                        FrontendMessage::Stop => {
                            let Some(bot) = &bot else {
                                break;
                            };
                            bot.stop();
                        }
                        FrontendMessage::NewPiece { piece } => {
                            let Some(bot) = &bot else {
                                break;
                            };
                            bot.add_piece(piece);
                        }
                        FrontendMessage::Play { mv: _ } => {
                            let Some(bot) = &bot else {
                                break;
                            };
                            for mv in &last_reply_moves {
                                bot.pick_move(*mv);
                            }
                        }
                        FrontendMessage::Suggest => {
                            let Some(bot) = &bot else {
                                break;
                            };
                            if let Some(plan) = bot.suggest() {
                                let mv = plan[0];
                                let piece = match mv {
                                    Move::Hold => {
                                        if let Some(&next_mv) = plan.get(1) {
                                            let Move::Place(next_piece) = next_mv else {
                                                panic!();
                                            };
                                            last_reply_moves = vec![mv, next_mv];
                                            next_piece
                                        } else {
                                            panic!();
                                        }
                                    }
                                    Move::Place(piece) => {
                                        last_reply_moves = vec![mv];
                                        piece
                                    }
                                };
                                let message = BotMessage::Suggestion {
                                    moves: vec![piece],
                                    move_info: MoveInfo {
                                        nodes: 0,
                                        nps: 0.0,
                                        extra: "".to_owned(),
                                    },
                                };
                                write_message(message);
                            }
                        }
                        FrontendMessage::Quit => break,
                        FrontendMessage::Unknown => break,
                    }
                }
            }
        })
}

fn write_message(message: BotMessage) {
    println!("{}", serde_json::to_string(&message).unwrap());
}
