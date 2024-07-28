use search::BotSync;

mod eval;
mod graph;
mod movegen;
mod search;

fn main() {
    let bot = BotSync::new();
    bot.init();
    // bot.work_loop();
    for i in 0..10000 {
        bot.search();
        if i % 100 == 0 {
            println!("{}, {:?}", i, bot.stats());
        }
    }
    println!("{:?}", bot.stats());
}
