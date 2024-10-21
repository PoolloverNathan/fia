# vim: ft=nix ts=2 sts=2 sw=2 et
# Adapted from https://mediocregopher.com/posts/x-compiling-rust-with-nix
{
  #{{{1 Inputs
  inputs.nixpkgs.url = "github:nixos/nixpkgs";
  inputs.fenix.url = "github:nix-community/fenix";
  inputs.naersk.url = "github:nix-community/naersk";
  #{{{1 Outputs
  outputs =
    {
      self,
      nixpkgs,
      fenix,
      naersk,
    }:
    rec {
      lib = rec {
        #{{{2 mkPackageSet
        mkPackageSet = mkPackage: rec {
          default = mkPackage [ ];
          withUnpack = mkPackage [ "unpack" ];
          withBackend = mkPackage [ "backend" ];
          full = mkPackage [ "full" ];
          inherit (default) cross;
        };
        #{{{2 buildTargets
        buildTargets = {
          x86_64-linux.crossSystemConfig = "x86_64-unknown-linux-musl";
          i686-linux.crossSystemConfig = "i686-unknown-linux-musl";
          aarch64-linux.crossSystemConfig = "aarch64-unknown-linux-musl";

          # Old Raspberry Pi's
          armv6l-linux = {
            crossSystemConfig = "armv6l-unknown-linux-musleabihf";
            rustTarget = "arm-unknown-linux-musleabihf";
            atomic = true;
          };

          x86_64-windows = {
            crossSystemConfig = "x86_64-w64-mingw32";
            rustTarget = "x86_64-pc-windows-gnu";
            makeBuildPackageAttrs = pkgsCross: {
              depsBuildBuild = [
                pkgsCross.stdenv.cc
                pkgsCross.windows.pthreads
              ];
            };
          };
        };
        #{{{2 perSystem
        perSystem = f: nixpkgs.lib.mapAttrs (system: f) nixpkgs.legacyPackages;
        #{{{2 toolchainFor
        toolchainFor =
          system: target:
          let
            toolchainData = {
              channel = "nightly";
              date = "2024-09-23";
              sha256 = "sha256:Dr3Ze+40mXcpM7RpYPU76+AFWLeI/ccW6F+TyZLERbg=";
            };
            fenixPkgs = fenix.packages.${system};
            hostToolchain = fenixPkgs.toolchainOf toolchainData;
          in
          fenixPkgs.combine [
            hostToolchain.rustc
            hostToolchain.cargo
            (fenixPkgs.targets.${target}.toolchainOf toolchainData).rust-std
          ];
        #{{{2 tagTrace
        /*
        tagTrace = tag: value: __trace "╔${tag}" (__deepSeq value (__trace "╚${tag}" value));
        tagTrace' = tag: value: __trace "┌${tag}" (__seq value (__trace "└${tag}" value));
        */
        tagTrace = tag: value: value;
        tagTrace' = tag: value: value;
        #{{{2 crossCompile
        crossCompile =
          {
            #{{{3 args
            src,
            pkgs,
            pkgsCross,
            toolchain,
            crossSystemConfig,
            rustTarget ? crossSystemConfig,
            buildPackageAttrs,
            atomic ? false,
            ...
          }:
          let
            #{{{3 bindings
            inherit (pkgsCross.pkgsStatic) openssl;
            inherit (pkgsCross.stdenv) cc;
            naersk-lib = tagTrace' "naersk" (
              pkgs.callPackage naersk {
                cargo = toolchain;
                rustc = toolchain;
              }
            );
            targetCC = tagTrace "targetCC" (pkgsCross.stdenv.cc + "/bin/${cc.targetPrefix}cc");
          in
          #{{{3 body
          tagTrace' "crossCompile" (
            tagTrace "buildPackage" naersk-lib.buildPackage (
              tagTrace' "buildPackageAttrs" buildPackageAttrs
              // tagTrace "moreBuildPackageAttrs" {
                src = tagTrace "src" src;
                strictDeps = true;
                OPENSSL_STATIC = true;
                OPENSSL_LIB_DIR = tagTrace "OPENSSL_LIB_DIR" (openssl.out + /lib);
                OPENSSL_INCLUDE_DIR = tagTrace "OPENSSL_INCLUDE_DIR" (openssl.dev + /include);
                TARGET_CC = tagTrace "TARGET_CC" targetCC; # ring requires special treatment
                CARGO_BUILD_TARGET = tagTrace "CARGO_BUILD_TARGET" rustTarget;
                CARGO_BUILD_RUSTFLAGS = tagTrace "CARGO_BUILD_RUSTFLAGS" [
                  "-C"
                  "target-feature=+crt-static"
                  "-C"
                  "link-args=-static${if atomic then " -latomic" else ""}"
                  "-C"
                  "linker=${targetCC}" # https://github.com/rust-lang/cargo/issues/4133
                ];
              }
            )
          );
        #{{{2 pkgsCrossFor
        pkgsCrossFor =
          system: target:
          tagTrace' "nixpkgs ${system}→${watson target "∅"}" (
            import nixpkgs (
              {
                inherit system;
              }
              // nixpkgs.lib.optionalAttrs (target != null) {
                crossSystem.config = tagTrace "crossSystem.config" buildTargets.${target}.crossSystemConfig;
              }
            )
          );
        #{{{2 watson
        watson = a: b: if a == null then b else a;
        #{{{2 crossCompile'
        crossCompile' =
          {
            #{{{3 args
            src,
            pkgs,
            target,
            attrs ? { },
          }:
          let
            #{{{3 bindings
            target' = watson target pkgs.system;
            buildTarget = tagTrace "buildTarget" buildTargets.${target'};
            pkgsCross = pkgsCrossFor pkgs.system target;
            toolchain = tagTrace' "toolchain" (
              toolchainFor pkgs.system (
                tagTrace' "toolchain-target" buildTarget.rustTarget or buildTarget.crossSystemConfig
              )
            );
            buildPackageAttrs = (
              tagTrace' "buildPackageAttrs =" (
                if buildTarget ? makeBuildPackageAttrs then
                  (tagTrace' "makeBuildPackageAttrs" (buildTarget.makeBuildPackageAttrs pkgsCross))
                else
                  { }
              )
            );
          in
          #{{{3 body
          tagTrace' "crossCompile'" (
            crossCompile (
              buildTarget
              // {
                inherit
                  src
                  pkgs
                  pkgsCross
                  toolchain
                  ;
                buildPackageAttrs = buildPackageAttrs // attrs;
              }
            )
          );
        #}}}2
      };
      formatter = lib.perSystem (pkgs: pkgs.nixfmt-rfc-style);
      packages = lib.perSystem (
        pkgs:
        let
          mkPackage =
            features:
            let
              f =
                target:
                lib.tagTrace' "crossCompile' return value" (
                  lib.crossCompile' {
                    src = ./.;
                    inherit pkgs target;
                    attrs.FOKDIR = ./foks;
                    attrs.cargoBuildFlags = lib.tagTrace "cargoBuildFlags" (
                      pkgs.lib.concatMap (feature: [
                        "--feature"
                        "fia/${feature}"
                      ]) features
                    );
                  }
                );
            in
            f null // { cross = lib.tagTrace' "cross" (pkgs.lib.mapAttrs f lib.buildTargets); };
        in
        (lib.tagTrace' "mkPackageSet return value" (
          (lib.tagTrace "mkPackageSet" lib.mkPackageSet) mkPackage
        ))
      );
    };
  #}}}1
}
