# Development shell: toolchain, clippy, rust-analyzer, pkg-config and the
# window-system libraries. `LD_LIBRARY_PATH` is set because winit and
# softbuffer dlopen libwayland/libxkbcommon/libX11 at run time.
{ lib, ... }:
{
  perSystem =
    { config, ... }:
    let
      cfg = config.lean;
      inherit (cfg) pkgs;
    in
    {
      devShells = {
        lean-browser = cfg.craneLib.devShell {
          # Pull in the build inputs of every check so `nix develop` can run
          # them all without extra tooling.
          checks = config.checks;

          packages =
            cfg.nativeBuildInputs
            ++ cfg.buildInputs
            ++ (with pkgs; [
              cargo-nextest
              xvfb-run # headless X for measuring the windowed renderer in CI
            ]);

          LD_LIBRARY_PATH = lib.makeLibraryPath cfg.buildInputs;
          RUST_SRC_PATH = "${cfg.toolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "lean-browser dev shell: $(rustc --version)"
          '';
        };

        default = config.devShells.lean-browser;
      };
    };
}
