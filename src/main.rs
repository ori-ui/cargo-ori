mod apk;

use std::{io, process};

use clap::{Parser, Subcommand};
use eyre::Context;
use serde::Deserialize;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let Options::Ori(options) = Options::parse();

    run_command(options.command)?;

    Ok(())
}

#[derive(Parser)]
enum Options {
    /// Ori is a tool for building ori projects.
    Ori(Ori),
}

#[derive(Parser)]
struct Ori {
    /// The subcommand to run.
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// APK is a tool for working with Android APKs.
    #[clap(subcommand)]
    Apk(apk::Command),
}

fn run_command(command: Command) -> eyre::Result<()> {
    match command {
        Command::Apk(command) => command.run(),
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct OriMetadata {
    pub name: Option<String>,
}

impl OriMetadata {
    pub fn from_package(package: &cargo_metadata::Package) -> eyre::Result<Self> {
        match package.metadata.get("ori") {
            Some(value) => Ok(serde_json::from_value(value.clone())?),
            None => Ok(Self::default()),
        }
    }
}

pub fn is_cross_installed() -> bool {
    let mut cmd = process::Command::new("cross");
    cmd.arg("--version");

    match cmd.output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub fn ensure_cross_installed() -> eyre::Result<()> {
    if is_cross_installed() {
        return Ok(());
    }

    println!("`cross` is not install, do you want to install it? [Y/n] ");

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;

    if answer.trim() == "n" || answer.trim() == "no" {
        eyre::bail!("`cross` is not installed");
    }

    let output = process::Command::new("cargo")
        .arg("--color")
        .arg("always")
        .arg("install")
        .arg("cross")
        .arg("--git")
        .arg("https://github.com/cross-rs/cross")
        .output()?;

    if !output.status.success() {
        eyre::bail!("`cross` could not be installed");
    }

    Ok(())
}

pub fn get_cargo_metadata() -> eyre::Result<cargo_metadata::Metadata> {
    let mut args = std::env::args().skip_while(|v| !v.starts_with("--manifest-path"));

    let mut cmd = cargo_metadata::MetadataCommand::new();
    match args.next() {
        Some(ref p) if p == "--manifest-path" => {
            cmd.manifest_path(args.next().unwrap());
        }
        Some(p) => {
            cmd.manifest_path(p.trim_start_matches("--manifest-path="));
        }
        None => {}
    };

    cmd.exec().wrap_err("Failed to get cargo metadata")
}
