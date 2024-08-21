use firefly::HikariFireflyBot;

fn main() {
    let bot = HikariFireflyBot::new();
    bot.start();
    std::thread::sleep(std::time::Duration::from_secs(60));
    bot.stop();
    std::thread::sleep(std::time::Duration::from_secs(1));
}
