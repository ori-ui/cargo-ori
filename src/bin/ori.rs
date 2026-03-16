use std::process::ExitCode;

use clap::Parser;
use owo_colors::OwoColorize;

fn main() -> ExitCode {
    let _ = color_eyre::config::HookBuilder::new().install();

    let args = cargo_ori::Args::parse();

    if let Err(err) = args.run() {
        eprintln!("{} {err}", "error:".red().bold());

        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
