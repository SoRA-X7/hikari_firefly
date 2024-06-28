use search::BotSync;

mod eval;
mod graph;
mod movegen;
mod search;

fn main() {
    let bot = BotSync::new();
    bot.work_loop();
}
