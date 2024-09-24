use std::{fs, path::Path};

use clap::Parser;

use crate::data::Replay;

#[derive(Debug, Parser)]
pub struct AnnotateArgs {
    #[clap(short, long)]
    cc_path: String,
    #[clap(short, long)]
    in_path: String,
    #[clap(short, long)]
    out_path: String,
}

pub fn annotate(args: AnnotateArgs) {
    eprintln!("cc_path: {}", args.cc_path);
    eprintln!("in_path: {}", args.in_path);
    eprintln!("out_path: {}", args.out_path);

    let in_path = Path::new(&args.in_path);

    if in_path.is_dir() {
        for entry in fs::read_dir(in_path).expect("read_dir call failed") {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    eprintln!("Processing file: {:?}", path);

                    // decompress msgpack
                    let replays: Vec<Replay> =
                        rmp_serde::from_slice(&fs::read(path).expect("unable to read"))
                            .expect("failed to decompress msgpack");

                    // annotate
                    replays.into_iter().for_each(|replay| {
                        let mut replay = replay;
                        annotate_replay(replay);
                    });
                }
            }
        }
    } else {
        panic!("The provided in_path is not a directory");
    }
}

fn annotate_replay(replay: Replay) {}
