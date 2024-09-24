use clap::Parser;

#[derive(Parser, Debug)]
pub struct GenArgs {
    #[clap(short, long)]
    cc_path: String,
    #[clap(short, long)]
    out_path: String,
}

pub fn generate(args: GenArgs) {
    println!("cc_path: {}", args.cc_path);
    println!("out_path: {}", args.out_path);
}
