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

        cargoSrc = mkOption {
          type = types.path;
          description = "Workspace source filtered to what cargo needs (Cargo.toml/lock, .rs, build scripts). Used for the dependency-only artifacts and the cargo-only checks so that a corpus or reference-image change does not rebuild third-party crates.";
          default = cfg.craneLib.cleanCargoSource ../.;
        };

        src = mkOption {
          type = types.path;
          description = "Workspace source plus the corpus, reference images, test fixtures, fonts and syntaxes; used by the derivations that run tests or the harness.";
          default = lib.cleanSourceWith {
            src = ../.;
            filter =
              path: type:
              (cfg.craneLib.filterCargoSources path type)
              || (lib.hasInfix "/corpus/" path)
              || (lib.hasInfix "/refs/" path)
              || (lib.hasInfix "/tests/fixtures/" path)
              || (lib.hasInfix "/fonts/" path)
              || (lib.hasInfix "/syntaxes/" path);
          };
        };

        testFonts = mkOption {
          type = types.package;
          description = "Font package the test check points LEAN_FONT_DIR at, so the shaping tests do not skip themselves inside the Nix sandbox (which has no system fonts).";
          default = pkgs.dejavu_fonts;
        };

        buildInputs = mkOption {
          type = types.listOf types.package;
          description = "Libraries winit/softbuffer/accesskit link or dlopen on Linux.";
          default = with pkgs; [
            libxkbcommon
            wayland
            libx11
            libxcursor
            libxi
            libxrandr
            libxcb
          ];
        };

        nativeBuildInputs = mkOption {
          type = types.listOf types.package;
          description = "Build-time tools.";
          default = [ pkgs.pkg-config ];
        };

        commonArgs = mkOption {
          type = types.attrs;
          description = "Arguments shared by every crane invocation (the full `src`; cargo-only derivations override `src` with `cargoSrc`).";
          default = {
            inherit (cfg) src buildInputs nativeBuildInputs;
            pname = "lean-browser";
            version = "0.1.0";
            strictDeps = true;
          };
        };

        cargoArtifacts = mkOption {
          type = types.package;
          description = "Dependency-only build (headless features) reused by packages and checks; built from `cargoSrc`.";
          default = cfg.craneLib.buildDepsOnly (
            cfg.commonArgs
            // {
              src = cfg.cargoSrc;
              cargoExtraArgs = "--locked --workspace";
            }
          );
        };

        cargoArtifactsWindow = mkOption {
          type = types.package;
          description = "Dependency-only build with `renderer/window` (winit, softbuffer, accesskit) for the windowed package.";
          default = cfg.craneLib.buildDepsOnly (
            cfg.commonArgs
            // {
              src = cfg.cargoSrc;
              pname = "lean-browser-window";
              cargoExtraArgs = "--locked --workspace --features renderer/window";
            }
          );
        };
      };
    }
  );
}
