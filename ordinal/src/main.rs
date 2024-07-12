use search::BotSync;

mod eval;
mod graph;
mod movegen;
mod search;

fn main() {
    let bot = BotSync::new();
    bot.init();
    // bot.work_loop();
    bot.search();
    bot.search();
    bot.search();
    bot.search();
}
