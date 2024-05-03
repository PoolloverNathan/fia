{ pkgs ? import <nixpkgs> {} }:
let mf = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = mf.name;
  version = mf.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;
  nativeBuildInputs = [pkgs.pkg-config pkgs.nix pkgs.openssl pkgs.openssl.dev];
  # OPENSSL_DIR = pkgs.openssl.dev;
  # OPENSSL_STATIC = true;
  # RUSTFLAGS = ["-v"  "-L" "${pkgs.openssl}/lib"];
  PKG_CONFIG_PATH = ["${pkgs.openssl.dev}/lib/pkgconfig/"];
}