{
  description = "Lean Browser: a limited web browser optimised for minimal resident private memory";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  # flake.nix is deliberately empty of logic: everything lives in flake-parts
  # modules under ./nix so the same modules can be imported by other flakes
  # through `flake.flakeModules.default`.
  outputs =
    inputs@{ flake-parts, ... }:
    let
      leanModule = import ./nix/flake-module.nix { leanInputs = inputs; };
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      imports = [ leanModule ];
      flake.flakeModules.default = leanModule;
    };
}
