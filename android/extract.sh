#!/bin/sh
gradle build -Dorg.gradle.project.android.aapt2FromMavenOverride=$(nix eval --raw nixpkgs#aapt.outPath)/bin/aapt2
nix-shell -p unzip --command "unzip library/build/outputs/apk/release/library-release-unsigned.apk -d apk"
cp apk/classes.dex ../src/classes.dex
rm -rf apk/
