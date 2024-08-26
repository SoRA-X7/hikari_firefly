use core::time;
use std::thread;

use game::{
    player::tetris::{NopInterface, TetrisPlayer},
    Game,
};

fn main() {
    let con1 = Box::new(NopInterface);
    let p1 = Box::new(TetrisPlayer::new(con1));
    let mut game = Game::new(vec![p1]);
    let mut tick = 0u64;

    loop {
        game.update();
        thread::sleep(time::Duration::from_micros(16666));
        eprintln!("{}", tick);
        tick += 1;
    }
}
