{ pkgs ? import <nixpkgs> {
  config = {
    android_sdk.accept_license = true;
    allowUnfree = true;
  };
} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.jdk
    pkgs.gradle
    pkgs.sdkmanager
    pkgs.android-studio
    pkgs.aapt
    pkgs.nix-ld
    pkgs.android-tools
  ];
}
