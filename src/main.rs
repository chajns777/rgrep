use std::env;
use std::process;
use rgrep::cli::{self, Config, ParseResult};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match Config::build(&args) {
        Ok(ParseResult::Run(cfg)) => cfg,
        Ok(ParseResult::Help) => {
            cli::print_help();
            return;
        }
        Err(err) => {
            eprintln!("Problem parsing arguments: {err}");
            process::exit(1);
        }
    };
    if let Err(e) = rgrep::run(config) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}