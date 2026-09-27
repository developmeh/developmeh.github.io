# Workspace packages built with crane from Cargo.lock.
{ ... }:
{
  perSystem =
    { config, ... }:
    let
      inherit (config.lean)
        craneLib
        commonArgs
        cargoArtifacts
        cargoArtifactsWindow
        ;
      # The binaries need only the cargo sources; `--locked` keeps crane's
      # default so a stale Cargo.lock fails instead of being rewritten.
      workspace = commonArgs // {
        src = config.lean.cargoSrc;
        doCheck = false; # tests run in checks.nix
      };
    in
    {
      packages = {
        # All binaries (lean-loader, lean-browser, lean-measure, lean-diff),
        # headless renderer only.
        lean-browser = craneLib.buildPackage (
          workspace
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--locked --workspace";
          }
        );

        # Same, with the winit/softbuffer/accesskit window enabled.
        lean-browser-window = craneLib.buildPackage (
          workspace
          // {
            pname = "lean-browser-window";
            cargoArtifacts = cargoArtifactsWindow;
            cargoExtraArgs = "--locked --workspace --features renderer/window";
          }
        );

        default = config.packages.lean-browser;
      };
    };
}
