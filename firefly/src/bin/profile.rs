use firefly::{BotConfig, HikariFireflyBot};

fn main() {
    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    let _puffin_server = puffin_http::Server::new(&server_addr).unwrap();
    eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
    puffin::set_scopes_on(true);
    puffin::GlobalProfiler::lock().new_frame();

    let config = BotConfig { num_workers: 4 };
    let bot = HikariFireflyBot::new(config);
    bot.start();

    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let plan = bot.suggest().unwrap();
        bot.pick_move(plan[0]);
        puffin::GlobalProfiler::lock().new_frame();
    }

    bot.stop();
}
