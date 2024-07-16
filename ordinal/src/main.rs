use search::BotSync;

mod eval;
mod graph;
mod movegen;
mod search;

fn main() {
    let bot = BotSync::new();
    bot.init();
    // bot.work_loop();
    for _ in 0..1000 {
        bot.search();
        println!("{:?}", bot.stats());
    }
}
