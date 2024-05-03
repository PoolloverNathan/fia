{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=9155c3d38dce4d0a718ea58fc4a73c8981041497";
    # flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default = import ./default.nix { pkgs = nixpkgs.legacyPackages.x86_64-linux; };
    packages.aarch64-linux.default = import ./default.nix { pkgs = nixpkgs.legacyPackages.aarch64-linux; };
  };
}
