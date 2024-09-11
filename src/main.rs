use std::{path::PathBuf, process};

use clap::{Parser, Subcommand};
use eyre::Context;
use serde::Deserialize;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let Options::Ori(options) = Options::parse();

    run_command(&options.command)?;

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
    Apk(ApkCommand),
}

#[derive(Subcommand)]
enum ApkCommand {
    /// Build an APK from a Cargo project.
    Build(ApkBuildOptions),
}

#[derive(Parser)]
struct ApkBuildOptions {
    /// Path to the android SDK root.
    #[clap(long)]
    sdk: Option<PathBuf>,

    /// Build the artifact in release mode, with optimizations.
    #[clap(short, long)]
    release: bool,

    /// Path to the PEM encoded RSA2048 signing key and certificate.
    #[clap(long)]
    pem: Option<PathBuf>,

    /// The target platform for the APK.
    #[clap(long)]
    target: Option<String>,

    /// Cargo package to build.
    #[clap(short, long)]
    package: Option<String>,

    /// Run without accessing the network.
    #[clap(long)]
    offline: bool,

    /// Features to enable.
    #[clap(short = 'F', long)]
    features: Vec<String>,

    /// Use verbose output.
    #[clap(short, long)]
    verbose: bool,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
struct MobileMetadata {
    name: Option<String>,
    package: Option<String>,
}

impl MobileMetadata {
    fn get(package: &cargo_metadata::Package) -> eyre::Result<Self> {
        match package.metadata.get("mobile") {
            Some(value) => Ok(serde_json::from_value(value.clone())?),
            None => Ok(Self::default()),
        }
    }
}

enum Orientation {}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
struct ApkMetadata {
    /// The version code of the APK.
    version_code: Option<u32>,

    /// The version name of the APK.
    version_name: Option<String>,

    uses_feature: Option<Vec<String>>,
    uses_permission: Option<Vec<String>>,
}

impl ApkMetadata {
    fn get(package: &cargo_metadata::Package) -> eyre::Result<Self> {
        match package.metadata.get("apk") {
            Some(value) => Ok(serde_json::from_value(value.clone())?),
            None => Ok(Self::default()),
        }
    }
}

fn run_command(command: &Command) -> eyre::Result<()> {
    match command {
        Command::Apk(command) => run_apk_command(command)?,
    }

    Ok(())
}

fn run_apk_command(command: &ApkCommand) -> eyre::Result<()> {
    match command {
        ApkCommand::Build(options) => build_apk(options)?,
    }

    Ok(())
}

fn cross_installed() -> eyre::Result<bool> {
    let output = process::Command::new("cross")
        .arg("--version")
        .output()
        .wrap_err("Failed to run cross")?;

    Ok(output.status.success())
}

fn build_apk(options: &ApkBuildOptions) -> eyre::Result<()> {
    if !cross_installed()? {
        eprintln!("`cross` is not installed. Install it with `cargo install cross`.");
        process::exit(1);
    }

    let metadata = cargo_metadata()?;
    let package = metadata
        .root_package()
        .ok_or_else(|| eyre::eyre!("No package"))?;

    let mobile_metadata = MobileMetadata::get(package)?;
    let apk_metadata = ApkMetadata::get(package)?;

    let mut manifest = apk::AndroidManifest::default();

    match mobile_metadata.package {
        Some(ref package) => manifest.package = Some(package.clone()),
        None => manifest.package = Some(package.name.clone()),
    }

    match apk_metadata.version_code {
        Some(version_code) => manifest.version_code = Some(version_code),
        None => manifest.version_code = Some(1),
    }

    match apk_metadata.version_name {
        Some(ref version_name) => manifest.version_name = Some(version_name.clone()),
        None => manifest.version_name = Some(package.version.to_string()),
    }

    for permission in apk_metadata.uses_permission.iter().flatten() {
        manifest.uses_permission.push(apk::manifest::Permission {
            name: permission.clone(),
            max_sdk_version: None,
        });
    }

    match mobile_metadata.name {
        Some(ref name) => manifest.application.label = Some(name.clone()),
        None => manifest.application.label = Some(package.name.clone()),
    }

    manifest.application.has_code = Some(false);

    let mut activity = apk::manifest::Activity {
        name: Some(format!("android.app.{}", package.name)),
        exported: Some(true),
        hardware_accelerated: Some(true),
        ..Default::default()
    };

    match mobile_metadata.name {
        Some(ref name) => activity.label = Some(name.clone()),
        None => activity.label = Some(package.name.clone()),
    }

    manifest.application.activities.push(activity);

    let mut apk = apk::Apk::new(PathBuf::from("test.apk"), manifest, true)
        .map_err(|e| eyre::eyre!("{}", e))?;

    let sdk = match options.sdk {
        Some(ref sdk) => sdk,
        None => eyre::bail!("sdk not set"),
    };

    apk.add_res(None, sdk).map_err(|e| eyre::eyre!("{}", e))?;

    Ok(())
}

fn cargo_metadata() -> eyre::Result<cargo_metadata::Metadata> {
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
