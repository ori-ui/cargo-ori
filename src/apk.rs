use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process,
};

use cargo_metadata::camino::Utf8Path;
use clap::{Parser, Subcommand};
use eyre::Context;
use serde::Deserialize;

use crate::OriMetadata;

const CLASSES_DEX: &[u8] = include_bytes!("classes.dex");

#[derive(Subcommand)]
pub enum Command {
    /// Build an APK from a Cargo project.
    Build(BuildOptions),

    /// Install an APK using adb.
    Install(BuildOptions),
}

impl Command {
    pub fn run(self) -> eyre::Result<()> {
        match self {
            Command::Build(options) => {
                let metadata = crate::get_cargo_metadata()?;
                let package = get_package(&metadata, options.package.as_deref())?;

                let ori_metadata = OriMetadata::from_package(package)?;
                let apk_metadata = Metadata::from_package(package)?;
                let manifest = apk_manifest(package, &ori_metadata, &apk_metadata)?;

                build_apk(&metadata, package, &apk_metadata, &manifest, &options)?;
            }

            Command::Install(mut options) => {
                let metadata = crate::get_cargo_metadata()?;
                let package = get_package(&metadata, options.package.as_deref())?;

                let devices = get_devices()?;
                let device = if devices.len() == 1 {
                    &devices[0]
                } else {
                    eyre::bail!("No device selected, use `--device`")
                };

                if options.target.is_none() {
                    options.target = Some(String::from(device.target_triple()));
                }

                let ori_metadata = OriMetadata::from_package(package)?;
                let apk_metadata = Metadata::from_package(package)?;
                let manifest = apk_manifest(package, &ori_metadata, &apk_metadata)?;

                install_apk(
                    &metadata,
                    package,
                    &apk_metadata,
                    &manifest,
                    device,
                    &options,
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Parser)]
pub struct BuildOptions {
    /// Path to the android SDK root.
    #[clap(long)]
    pub sdk: Option<PathBuf>,

    /// Build the artifact in release mode, with optimizations.
    #[clap(short, long)]
    pub release: bool,

    /// Path to the PEM encoded RSA2048 signing key and certificate.
    #[clap(long)]
    pub pem: Option<PathBuf>,

    /// The target platform for the APK.
    #[clap(long)]
    pub target: Option<String>,

    /// Cargo package to build.
    #[clap(short, long)]
    pub package: Option<String>,

    /// Run without accessing the network.
    #[clap(long)]
    pub offline: bool,

    /// Features to enable.
    #[clap(short = 'F', long)]
    pub features: Vec<String>,

    /// Use verbose output.
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
struct Metadata {
    package: Option<String>,

    /// The version code of the APK.
    version_code: Option<u32>,

    /// The version name of the APK.
    version_name: Option<String>,

    /// The icon of the APK.
    icon: Option<String>,

    #[serde(default)]
    uses_feature: Vec<String>,

    #[serde(default)]
    uses_permission: Vec<String>,
}

impl Metadata {
    fn from_package(package: &cargo_metadata::Package) -> eyre::Result<Self> {
        match package.metadata.get("apk") {
            Some(value) => Ok(serde_json::from_value(value.clone())?),
            None => Ok(Self::default()),
        }
    }
}

struct Device {
    id: String,
    arch: apk::Target,
}

impl Device {
    fn target_triple(&self) -> &'static str {
        match self.arch {
            apk::Target::Arm64V8a => "aarch64-linux-android",
            apk::Target::ArmV7a => "arm7-linux-androideabi",
            apk::Target::X86 => "i686-linux-android",
            apk::Target::X86_64 => "x86_64-linux-android",
        }
    }
}

fn get_devices() -> eyre::Result<Vec<Device>> {
    let output = process::Command::new("adb").arg("devices").output()?;

    let mut devices = Vec::new();

    let mut lines = output.stdout.lines();
    lines.next();

    for line in lines {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        let id = line
            .split_whitespace()
            .next()
            .ok_or_else(|| eyre::eyre!("Malformed adb output"))?;

        let output = process::Command::new("adb")
            .arg("-s")
            .arg(id)
            .arg("shell")
            .arg("getprop")
            .arg("ro.product.cpu.abi")
            .output()?;

        let arch = String::from_utf8(output.stdout)?;

        let arch = match arch.trim() {
            "arm64-v8a" => apk::Target::Arm64V8a,
            "armabi-v7a" => apk::Target::ArmV7a,
            "x86_64" => apk::Target::X86_64,
            "x86" => apk::Target::X86,
            _ => eyre::bail!("Unknown abi `{}`", arch.trim()),
        };

        devices.push(Device {
            id: String::from(id),
            arch,
        });
    }

    Ok(devices)
}

fn get_package<'a>(
    metadata: &'a cargo_metadata::Metadata,
    name: Option<&str>,
) -> eyre::Result<&'a cargo_metadata::Package> {
    match name {
        Some(ref package) => metadata
            .packages
            .iter()
            .find(|p| p.name == *package)
            .ok_or_else(|| eyre::eyre!("Package `{}` not found", package)),
        None => metadata
            .root_package()
            .ok_or_else(|| eyre::eyre!("No package")),
    }
}

fn install_apk(
    metadata: &cargo_metadata::Metadata,
    package: &cargo_metadata::Package,
    apk_metadata: &Metadata,
    manifest: &apk::AndroidManifest,
    device: &Device,
    options: &BuildOptions,
) -> eyre::Result<()> {
    ensure_adb_installed()?;

    let apk_path = build_apk(metadata, package, apk_metadata, manifest, options)?;

    let output = process::Command::new("adb")
        .arg("-s")
        .arg(&device.id)
        .arg("install")
        .arg(apk_path)
        .output()?;

    if !output.status.success() {
        eyre::bail!("Install failed");
    }

    Ok(())
}

fn build_apk(
    metadata: &cargo_metadata::Metadata,
    package: &cargo_metadata::Package,
    apk_metadata: &Metadata,
    manifest: &apk::AndroidManifest,
    options: &BuildOptions,
) -> eyre::Result<PathBuf> {
    crate::ensure_cross_installed()?;

    let target = options
        .target
        .as_deref()
        .ok_or_else(|| eyre::eyre!("Target not specified, use `--target` to do so"))?;

    let apk_target = match target {
        "aarch64-linux-android" => apk::Target::Arm64V8a,
        "arm7-linux-androidabi" => apk::Target::ArmV7a,
        "x86_64-linux-android" => apk::Target::X86_64,
        "i686-linux-android" => apk::Target::X86,
        _ => eyre::bail!("Target '{}' is not supported for android", target),
    };

    let icon_path = apk_metadata
        .icon
        .as_ref()
        .map(|icon| metadata.workspace_root.join(icon));

    let artifact = build_lib(
        package,
        target,
        &options.features,
        options.release,
        options.offline,
    )?;
    let sdk_path = download_android_sdk(&metadata.target_directory, 34)?;

    let lib_path = artifact_cdylib(&artifact)?.strip_prefix("/")?;
    let lib_path = metadata.workspace_root.join(lib_path);

    let lib_parent = lib_path.parent().expect("lib_path has parent");

    let apk_path: PathBuf = lib_parent.join(format!("{}.apk", package.name)).into();

    let dex_path = sdk_path
        .parent()
        .expect("sdk_path has parent")
        .join("classes.dex");

    fs::write(&dex_path, CLASSES_DEX).wrap_err("Failed to write classes.dex")?;

    let mut apk = apk::Apk::new(apk_path.clone(), manifest.clone(), true)
        .map_err(|e| eyre::eyre!("{}", e))?;

    apk.add_res(icon_path.as_ref().map(AsRef::as_ref), sdk_path.as_ref())
        .map_err(|e| eyre::eyre!("{}", e))?;

    apk.add_dex(dex_path.as_ref())
        .map_err(|e| eyre::eyre!("{}", e))?;

    apk.add_lib(apk_target, lib_path.as_ref())
        .map_err(|e| eyre::eyre!("{}", e))?;

    let pem = match options.pem {
        Some(ref pem) => fs::read_to_string(pem).wrap_err("Failed to load PEM file")?,
        None => String::from(include_str!("debug.pem")),
    };

    let signer = apk::Signer::new(&pem).map_err(|e| eyre::eyre!("{}", e))?;

    apk.finish(Some(signer)).map_err(|e| eyre::eyre!("{}", e))?;

    Ok(apk_path)
}

fn build_lib(
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
        match message? {
            cargo_metadata::Message::CompilerArtifact(artifact) => {
                if artifact.package_id == package.id {
                    package_artifact = Some(artifact);
                }
            }
            cargo_metadata::Message::CompilerMessage(message) => {
                println!("{}", message.message);
            }
            cargo_metadata::Message::BuildScriptExecuted(_) => {}
            cargo_metadata::Message::BuildFinished(_) => {}
            cargo_metadata::Message::TextLine(line) => {
                println!("{}", line);
            }
            _ => {}
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

fn apk_manifest(
    package: &cargo_metadata::Package,
    ori_metadata: &OriMetadata,
    apk_metadata: &Metadata,
) -> eyre::Result<apk::AndroidManifest> {
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

    match apk_metadata.package {
        Some(ref package) => manifest.package = Some(package.clone()),
        None => manifest.package = Some(format!(".{}", package.name.replace("-", "_"))),
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

    match ori_metadata.name {
        Some(ref name) => manifest.application.label = Some(name.clone()),
        None => manifest.application.label = Some(package.name.clone()),
    }

    manifest.application.theme = Some(String::from(
        "@android:style/Theme.DeviceDefault.NoActionBar.TranslucentDecor",
    ));

    let mut activity = apk::manifest::Activity {
        name: Some(String::from("ori.oriactivity.OriActivity")),
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

    match ori_metadata.name {
        Some(ref name) => activity.label = Some(name.clone()),
        None => activity.label = Some(package.name.clone()),
    }

    manifest.application.activities.push(activity);

    Ok(manifest)
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

fn is_adb_installed() -> bool {
    let mut cmd = process::Command::new("adb");
    cmd.arg("version");

    match cmd.output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn ensure_adb_installed() -> eyre::Result<()> {
    if !is_adb_installed() {
        eyre::bail!("`adb` is not installed");
    }

    Ok(())
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
