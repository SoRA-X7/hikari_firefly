use annotate::{annotate, AnnotateArgs};
use clap::{Parser, Subcommand};
use generate::{generate, GenArgs};

mod annotate;
mod cc;
mod data;
mod generate;

/// Hikari learning data generator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    Gen(GenArgs),
    Annotate(AnnotateArgs),
}

fn main() {
    let args = Args::parse();
    match args.subcmd {
        SubCommand::Gen(gen_args) => generate(gen_args),
        SubCommand::Annotate(annotate_args) => annotate(annotate_args),
    }
}
