# Declares the per-system `lean.*` options every other module reads:
# the Rust toolchain, the crane library bound to it, the filtered source,
# the native/runtime libraries winit needs, and the shared crane arguments
# (including the pre-built dependency artifacts).
{ leanInputs }:
{ lib, ... }:
let
  inherit (lib) mkOption types;
  inherit (leanInputs.flake-parts.lib) mkPerSystemOption;
in
{
  options.perSystem = mkPerSystemOption (
    { config, system, ... }:
    let
      cfg = config.lean;
      pkgs = cfg.pkgs;
      rustPkgs = pkgs.extend leanInputs.rust-overlay.overlays.default;
    in
    {
      options.lean = {
        pkgs = mkOption {
          type = types.pkgs;
          description = "Package set used for everything below; defaults to this flake's locked nixpkgs so importers get reproducible builds.";
          default = leanInputs.nixpkgs.legacyPackages.${system};
        };

        toolchain = mkOption {
          type = types.package;
          description = "Rust toolchain (rustc, cargo, clippy, rustfmt, rust-analyzer, rust-src).";
          default = rustPkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
              "rustfmt"
            ];
          };
        };

        craneLib = mkOption {
          type = types.attrs;
          description = "crane library using `lean.toolchain`.";
          default = (leanInputs.crane.mkLib pkgs).overrideToolchain cfg.toolchain;
        };

        src = mkOption {
          type = types.path;
          description = "Workspace source, filtered to what cargo needs plus the corpus and reference images.";
          default = lib.cleanSourceWith {
            src = ../.;
            filter =
              path: type:
              (cfg.craneLib.filterCargoSources path type)
              || (lib.hasInfix "/corpus/" path)
              || (lib.hasInfix "/refs/" path)
              || (lib.hasInfix "/fonts/" path)
              || (lib.hasInfix "/syntaxes/" path);
          };
        };

        buildInputs = mkOption {
          type = types.listOf types.package;
          description = "Libraries winit/softbuffer/accesskit link or dlopen on Linux.";
          default = with pkgs; [
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xorg.libxcb
          ];
        };

        nativeBuildInputs = mkOption {
          type = types.listOf types.package;
          description = "Build-time tools.";
          default = [ pkgs.pkg-config ];
        };

        commonArgs = mkOption {
          type = types.attrs;
          description = "Arguments shared by every crane invocation.";
          default = {
            inherit (cfg) src buildInputs nativeBuildInputs;
            pname = "lean-browser";
            version = "0.1.0";
            strictDeps = true;
          };
        };

        cargoArtifacts = mkOption {
          type = types.package;
          description = "Dependency-only build reused by packages and checks.";
          default = cfg.craneLib.buildDepsOnly cfg.commonArgs;
        };
      };
    }
  );
}
