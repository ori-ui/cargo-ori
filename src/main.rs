mod build;
mod devices;
mod init;
mod metadata;
mod run;

use std::process::ExitCode;

use clap::Parser;
use owo_colors::colored::OwoColorize;

fn main() -> ExitCode {
    let _ = color_eyre::config::HookBuilder::new().install();

    let Ori::Ori(args) = Ori::parse();

    if let Err(err) = args.run() {
        eprintln!("{} {err}", "error:".red().bold());

        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(Parser)]
enum Ori {
    /// Build and run ori projects.
    Ori(Args),
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

impl Args {
    fn run(self) -> eyre::Result<()> {
        match self.command {
            Command::Init(init) => init.run(),
            Command::Devices(devices) => devices.run(),
            Command::Build(build) => build.run(),
            Command::Run(run) => run.run(),
        }
    }
}

#[derive(Parser)]
enum Command {
    /// Initialize a project.
    Init(init::Command),

    Devices(devices::Command),

    /// Build a project.
    #[clap(visible_alias = "b")]
    Build(build::Command),

    /// Run a project.
    #[clap(visible_alias = "r")]
    Run(run::Command),
}
