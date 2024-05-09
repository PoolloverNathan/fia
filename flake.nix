{
  inputs.stddev.url = "github:PoolloverNathan/stddev";
  outputs = {
    self,
    stddev,
  }:
    stddev {
      name = "fia";
      packages = system: pkgs: {
        default = let
          mf = (pkgs.lib.importTOML ./Cargo.toml).package;
        in
          pkgs.rustPlatform.buildRustPackage rec {
            pname = mf.name;
            version = mf.version;
            cargoLock.lockFile = ./Cargo.lock;
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = [pkgs.pkg-config pkgs.nix pkgs.openssl pkgs.openssl.dev];
            PKG_CONFIG_PATH = ["${pkgs.openssl.dev}/lib/pkgconfig/"];
          };
      };
    };
}
