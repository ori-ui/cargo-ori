use std::{env, ffi::OsString, path::PathBuf, process};

use cargo_metadata::{Metadata, Package};
use eyre::OptionExt;
use owo_colors::OwoColorize;

pub struct Android {
    pub root_package: Package,
    pub android_directory: PathBuf,
    pub package: String,
    pub activity: String,
    pub gradle: OsString,
}

impl Android {
    fn get_root_directory(cargo: &Metadata) -> eyre::Result<PathBuf> {
        let root_package = cargo
            .root_package()
            .ok_or_eyre("root package could not be found")?;

        Ok(root_package
            .manifest_path
            .parent()
            .expect("files always have a parent directory")
            .into())
    }

    fn get_android_directory(cargo: &Metadata) -> eyre::Result<PathBuf> {
        let android_directory = Self::get_root_directory(cargo)?.join("android");

        if !android_directory.exists() {
            eyre::bail!("package does not have an android project");
        }

        Ok(android_directory)
    }

    pub fn get_gradle(cargo: &Metadata) -> eyre::Result<OsString> {
        let android_directory = Self::get_root_directory(cargo)?;

        let gradlew = if cfg!(target_os = "windows") {
            android_directory.join("gradlew.bat")
        } else {
            android_directory.join("gradlew")
        };

        let gradle = if gradlew.exists() {
            gradlew.as_os_str().to_owned()
        } else {
            OsString::from("gradle")
        };

        if process::Command::new(&gradle)
            .arg("--version")
            .output()
            .is_err()
        {
            eyre::bail!("a valid version of `gradle` could not be found");
        }

        Ok(gradle)
    }

    pub fn get_adb(cargo: &Metadata) -> eyre::Result<OsString> {
        let adb = if let Ok(sdk_root) = env::var("ANDROID_SDK_ROOT") {
            format!("{sdk_root}/platform-tools/adb{}", env::consts::EXE_SUFFIX)
        } else if let Ok(adb) = Self::get_adb_from_gradle(cargo) {
            adb
        } else {
            String::from("adb")
        };

        if process::Command::new(&adb)
            .arg("--version")
            .output()
            .is_err()
        {
            eyre::bail!("a valid version of `adb` could not be found");
        }

        Ok(adb.into())
    }

    fn get_adb_from_gradle(cargo: &Metadata) -> eyre::Result<String> {
        let gradle = Self::get_gradle(cargo)?;
        let android_directory = Self::get_android_directory(cargo)?;

        let get_adb_output = process::Command::new(gradle)
            .current_dir(android_directory)
            .arg("getAdbExe")
            .arg("--quiet")
            .output()?;

        if get_adb_output.status.success() {
            Ok(String::from_utf8_lossy(&get_adb_output.stderr).to_string())
        } else {
            Err(eyre::eyre!("failed to get adb executable from gradle"))
        }
    }

    pub fn new(cargo: &Metadata) -> eyre::Result<Self> {
        let root_package = cargo
            .root_package()
            .ok_or_eyre("root package could not be found")?;

        let root_directory = root_package
            .manifest_path
            .parent()
            .expect("files always have a parent directory");

        let android_directory = root_directory.join("android");

        if !android_directory.exists() {
            eyre::bail!(
                "package `{}` does not have an android project",
                root_package.name,
            );
        }

        let android_metadata = root_package
            .metadata
            .get("android")
            .ok_or_eyre("could not find `package.metadata.android` in Cargo.toml")?;

        Self::check(android_metadata);

        let package = android_metadata
            .get("package")
            .ok_or_eyre("could not find `package.metadata.android.package` in Cargo.toml")?
            .as_str()
            .ok_or_eyre("`android.package` must be a string")?;

        let activity = format!("{package}/ori.OriActivity");

        let gradle = Self::get_gradle(cargo)?;

        Ok(Self {
            root_package: root_package.clone(),
            android_directory: android_directory.into(),
            package: package.into(),
            activity,
            gradle,
        })
    }

    pub fn check(metadata: &serde_json::Value) {
        const VALID_KEYS: &[&str] = &[
            "name",
            "key-properties",
            "package",
            "targets",
            "version",
            "version-code",
        ];

        let Some(object) = metadata.as_object() else {
            return;
        };

        for key in object.keys() {
            if !VALID_KEYS.contains(&key.as_str()) {
                eprintln!(
                    "{} unused key `package.metadata.android.{key}`",
                    "warning:".yellow().bold(),
                );
            }
        }

        if let Some(targets) = object.get("targets") {
            Self::check_targets(targets);
        }
    }

    fn check_targets(targets: &serde_json::Value) {
        const VALID_TARGETS: &[&str] = &["arm64-v8a", "armabi-v7a", "x86_64", "x86"];

        let Some(array) = targets.as_array() else {
            eprintln!(
                "{} `package.metadata.android.targets` must be a list",
                "error:".red().bold(),
            );

            return;
        };

        if array.is_empty() {
            eprintln!(
                "{} no android targets specified",
                "warning:".yellow().bold(),
            );
        }

        for target in array {
            let Some(target) = target.as_str() else {
                eprintln!(
                    "{} `package.metadata.android.targets` must be a list of strings",
                    "error:".red().bold(),
                );

                continue;
            };

            if !VALID_TARGETS.contains(&target) {
                eprintln!(
                    "{} invalid android target `{}`, valid targets are [{}]",
                    "warning:".yellow().bold(),
                    target,
                    VALID_TARGETS.join(", "),
                );
            }
        }
    }
}
