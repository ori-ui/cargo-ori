use std::{ffi::OsString, path::PathBuf};

use cargo_metadata::{Metadata, Package};
use eyre::OptionExt;

pub struct Android {
    pub root_package: Package,
    pub android_directory: PathBuf,
    pub package: String,
    pub activity: String,
    pub gradle: OsString,
}

impl Android {
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
            "compile-sdk",
            "target-sdk",
            "min-sdk",
            "version",
            "version-code",
        ];

        let Some(object) = metadata.as_object() else {
            return;
        };

        for key in object.keys() {
            if !VALID_KEYS.contains(&key.as_str()) {
                eprintln!("unused key `package.metadata.android.{key}`");
            }
        }
    }
}
