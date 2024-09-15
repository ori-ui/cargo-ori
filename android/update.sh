#!/bin/sh
rm -rf apk/
gradle build -Dorg.gradle.project.android.aapt2FromMavenOverride=$(nix eval --raw nixpkgs#aapt.outPath)/bin/aapt2
nix-shell -p unzip --command "unzip library/build/outputs/apk/release/library-release-unsigned.apk -d apk"
cp apk/classes.dex ../../dew/target/apk/platforms/android-34/classes.dex
