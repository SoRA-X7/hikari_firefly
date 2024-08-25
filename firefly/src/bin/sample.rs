use firefly::HikariFireflyBot;

fn main() {
    let bot = HikariFireflyBot::new();
    bot.start();

    for _ in 0..11 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let plan = bot.suggest().unwrap();
        bot.pick_move(plan[0]);
    }

    bot.stop();
    std::thread::sleep(std::time::Duration::from_secs(1));
}
