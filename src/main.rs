use std::{io::BufReader, path::PathBuf, process};

use cargo_metadata::camino::Utf8Path;
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
    target: String,

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

    #[serde(default)]
    uses_feature: Vec<String>,

    #[serde(default)]
    uses_permission: Vec<String>,
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

    let apk_target = match options.target.as_str() {
        "aarch64-linux-android" => apk::Target::Arm64V8a,
        "arm7-linux-androidabi" => apk::Target::ArmV7a,
        "x86_64-linux-android" => apk::Target::X86_64,
        "i686-linux-android" => apk::Target::X86,
        _ => eyre::bail!("Target '{}' is not supported for android", options.target),
    };

    let metadata = cargo_metadata()?;
    let package = match options.package {
        Some(ref package) => metadata
            .packages
            .iter()
            .find(|p| p.name == *package)
            .ok_or_else(|| eyre::eyre!("Package `{}` not found", package))?,
        None => metadata
            .root_package()
            .ok_or_else(|| eyre::eyre!("No package"))?,
    };

    let artifact = build_android(
        package,
        &options.target,
        &options.features,
        options.release,
        options.offline,
    )?;
    let sdk_path = download_android_sdk(&metadata.target_directory, 34)?;

    let mobile_metadata = MobileMetadata::get(package)?;
    let apk_metadata = ApkMetadata::get(package)?;

    let mut manifest = apk::AndroidManifest::default();

    let version = 34;
    let version_code = 14;
    let min_version = 21;

    manifest.compile_sdk_version = Some(version);
    manifest.platform_build_version_code = Some(version);
    manifest.compile_sdk_version_codename = Some(version_code);
    manifest.platform_build_version_name = Some(version_code);
    manifest.sdk.target_sdk_version = Some(version);
    manifest.sdk.min_sdk_version = Some(min_version);

    match mobile_metadata.package {
        Some(ref package) => manifest.package = Some(package.clone()),
        None => manifest.package = Some(format!("org.{}", package.name.replace("-", "_"))),
    }

    match apk_metadata.version_code {
        Some(version_code) => manifest.version_code = Some(version_code),
        None => manifest.version_code = Some(1),
    }

    match apk_metadata.version_name {
        Some(ref version_name) => manifest.version_name = Some(version_name.clone()),
        None => manifest.version_name = Some(package.version.to_string()),
    }

    for feature in apk_metadata.uses_feature.iter() {
        manifest.uses_feature.push(apk::manifest::Feature {
            name: Some(feature.clone()),
            required: None,
            version: None,
            opengles_version: None,
        });
    }

    for permission in apk_metadata.uses_permission.iter() {
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
    manifest.application.theme = Some(String::from(
        "@android:style/Theme.DeviceDefault.NoActionBar.TranslucentDecor",
    ));

    let mut activity = apk::manifest::Activity {
        name: Some(String::from("android.app.NativeActivity")),
        exported: Some(true),
        hardware_accelerated: Some(true),
        meta_data: vec![apk::manifest::MetaData {
            name: String::from("android.app.lib_name"),
            value: package.name.replace("-", "_"),
        }],
        intent_filters: vec![apk::manifest::IntentFilter {
            actions: vec![String::from("android.intent.action.MAIN")],
            categories: vec![String::from("android.intent.category.LAUNCHER")],
            ..Default::default()
        }],
        config_changes: Some(
            [
                "orientation",
                "keyboardHidden",
                "keyboard",
                "screenSize",
                "smallestScreenSize",
                "locale",
                "layoutDirection",
                "fontScale",
                "screenLayout",
                "density",
                "uiMode",
            ]
            .join("|"),
        ),
        launch_mode: Some(String::from("singleTop")),
        window_soft_input_mode: Some(String::from("adjustResize")),
        ..Default::default()
    };

    match mobile_metadata.name {
        Some(ref name) => activity.label = Some(name.clone()),
        None => activity.label = Some(package.name.clone()),
    }

    manifest.application.activities.push(activity);

    let lib_path = artifact_cdylib(&artifact)?.strip_prefix("/")?;
    let lib_path = metadata.workspace_root.join(lib_path);

    let apk_path = lib_path
        .parent()
        .expect("lib_path has parent")
        .join(format!("{}.apk", package.name))
        .into();

    let mut apk = apk::Apk::new(apk_path, manifest, true).map_err(|e| eyre::eyre!("{}", e))?;

    apk.add_res(None, sdk_path.as_ref())
        .map_err(|e| eyre::eyre!("{}", e))?;

    apk.add_lib(apk_target, lib_path.as_ref())
        .map_err(|e| eyre::eyre!("{}", e))?;

    apk.finish(None).map_err(|e| eyre::eyre!("{}", e))?;

    Ok(())
}

fn build_android(
    package: &cargo_metadata::Package,
    target: &str,
    features: &[String],
    release: bool,
    offline: bool,
) -> eyre::Result<cargo_metadata::Artifact> {
    let mut command = process::Command::new("cross");

    command
        .arg("--color")
        .arg("always")
        .arg("build")
        .arg("--target")
        .arg(target)
        .arg("--message-format=json")
        .arg("--package")
        .arg(&package.name)
        .arg("--lib");

    if release {
        command.arg("--release");
    }

    if offline {
        command.arg("--offline");
    }

    if !features.is_empty() {
        command.arg("--features");
        command.arg(features.join(","));
    }

    let process = command
        .stdout(process::Stdio::piped())
        .spawn()
        .wrap_err("Failed to run cross")?;

    let reader = BufReader::new(process.stdout.expect("stdout available"));

    let mut package_artifact = None;

    for message in cargo_metadata::Message::parse_stream(reader) {
        if let cargo_metadata::Message::CompilerArtifact(artifact) = message? {
            if artifact.package_id == package.id {
                package_artifact = Some(artifact);
            }
        }
    }

    package_artifact.ok_or_else(|| eyre::eyre!("Artifact not generated"))
}

fn artifact_cdylib(artifact: &cargo_metadata::Artifact) -> eyre::Result<&Utf8Path> {
    let index = artifact
        .target
        .crate_types
        .iter()
        .position(|t| t == "cdylib")
        .ok_or_else(|| eyre::eyre!("No cdylib built"))?;

    Ok(&artifact.filenames[index])
}

fn download_android_sdk(target_directory: &Utf8Path, version: u32) -> eyre::Result<PathBuf> {
    let apk_dir = target_directory.join("apk");
    let android = format!("android-{}", version);
    let apk_path = apk_dir.join("platforms").join(&android).join("android.jar");

    if apk_path.exists() {
        return Ok(apk_path.into());
    }

    android_sdkmanager::download_and_extract_packages(
        apk_dir.as_str(),
        android_host_os(),
        &[&format!("platforms;{}", android)],
        Some(&[android_sdkmanager::MatchType::EntireName("android.jar")]),
    );

    Ok(apk_path.into())
}

fn android_host_os() -> android_sdkmanager::HostOs {
    if cfg!(target_os = "linux") {
        android_sdkmanager::HostOs::Linux
    } else if cfg!(target_os = "windows") {
        android_sdkmanager::HostOs::Windows
    } else if cfg!(target_os = "macos") {
        android_sdkmanager::HostOs::MacOs
    } else {
        panic!("Host os not supported")
    }
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
