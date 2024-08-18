use search::{bot_run, BotSync};

mod eval;
mod graph;
mod movegen;
mod search;
mod tbp;

// fn main() {
//     let bot = BotSync::new();
//     bot.init();
//     // bot.work_loop();
//     for i in 0..10000 {
//         bot.search();
//         if i % 100 == 0 {
//             println!("{}, {:?}", i, bot.stats());
//         }
//     }
//     println!("{:?}", bot.stats());
// }

fn main() {
    #[cfg(feature = "puffin_http")]
    let _puffin_server = match options.profile {
        true => {
            puffin::set_scopes_on(true);
            Some(puffin_http::Server::new(&format!(
                "0.0.0.0:{}",
                puffin_http::DEFAULT_PORT
            )))
        }
        false => None,
    };

    let incoming = futures::stream::repeat_with(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    });

    let outgoing = futures::sink::unfold((), |_, msg| {
        serde_json::to_writer(std::io::stdout(), &msg).unwrap();
        println!();
        async { Ok(()) }
    });

    futures::pin_mut!(incoming);
    futures::pin_mut!(outgoing);

    futures::executor::block_on(bot_run(incoming, outgoing));
}
