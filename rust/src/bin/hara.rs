#[path = "hara/cli.rs"]
mod cli;
#[path = "hara/repl.rs"]
mod repl;
#[path = "hara/terminal.rs"]
mod terminal;

fn main() {
    let options = match cli::parse_options() {
        Ok(options) => options,
        Err(error) => cli::exit_error(&error, 2),
    };
    if let Err(error) = cli::run(options) {
        cli::exit_error(&error, 1);
    }
}
