use std::process::ExitCode;

use clap::Parser;
use owo_colors::OwoColorize;

fn main() -> ExitCode {
    let _ = color_eyre::config::HookBuilder::new().install();

    let Command::Ori(args) = Command::parse();

    if let Err(err) = args.run() {
        eprintln!("{} {err}", "error:".red().bold());

        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(Parser)]
#[clap(styles = clap_cargo::style::CLAP_STYLING, bin_name = "cargo")]
enum Command {
    /// Build and run ori projects.
    Ori(cargo_ori::Args),
}
