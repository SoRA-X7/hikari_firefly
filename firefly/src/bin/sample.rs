use firefly::{BotConfig, HikariFireflyBot};
use game::tetris::GameState;

fn main() {
    let config = BotConfig { num_workers: 1 };
    let bot = HikariFireflyBot::new(config);

    let mut state = GameState::new();

    for _ in 0..12 {
        state.fulfill_queue();
    }

    bot.reset(Some(state.clone()));

    bot.start();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let plan = bot.suggest().unwrap();
        let mv = plan[0];
        eprintln!("Move: {:?}", mv);

        state.advance(mv);
        bot.pick_move(mv);

        let piece_appended = state.fulfill_queue();
        bot.add_piece(piece_appended);
        eprintln!("State: {:?}", state);
    }

    bot.stop();
    std::thread::sleep(std::time::Duration::from_secs(1));
}
