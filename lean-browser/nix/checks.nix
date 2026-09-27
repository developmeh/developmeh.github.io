# `nix flake check`: clippy with warnings denied, the test suite, rustfmt,
# and the package build itself.
{ ... }:
{
  perSystem =
    { config, ... }:
    let
      inherit (config.lean) craneLib commonArgs cargoArtifacts;
      withDeps = commonArgs // {
        inherit cargoArtifacts;
      };
    in
    {
      checks = {
        lean-browser-build = config.packages.lean-browser;

        lean-browser-clippy = craneLib.cargoClippy (
          withDeps
          // {
            cargoClippyExtraArgs = "--workspace --all-targets -- -D warnings";
          }
        );

        lean-browser-test = craneLib.cargoTest (
          withDeps
          // {
            cargoTestExtraArgs = "--workspace";
          }
        );

        lean-browser-fmt = craneLib.cargoFmt {
          inherit (config.lean) src;
        };
      };
    };
}
