# Workspace packages built with crane from Cargo.lock.
{ ... }:
{
  perSystem =
    { config, ... }:
    let
      inherit (config.lean) craneLib commonArgs cargoArtifacts;
      workspace = commonArgs // {
        inherit cargoArtifacts;
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
            cargoExtraArgs = "--workspace";
          }
        );

        # Same, with the winit/softbuffer/accesskit window enabled.
        lean-browser-window = craneLib.buildPackage (
          workspace
          // {
            pname = "lean-browser-window";
            cargoExtraArgs = "--workspace --features renderer/window";
          }
        );

        default = config.packages.lean-browser;
      };
    };
}
