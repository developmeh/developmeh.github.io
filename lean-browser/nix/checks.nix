# `nix flake check`: clippy with warnings denied, the test suite, rustfmt,
# and the package build itself.
{ ... }:
{
  perSystem =
    { config, ... }:
    let
      inherit (config.lean)
        craneLib
        commonArgs
        cargoArtifacts
        cargoSrc
        testFonts
        ;
      withDeps = commonArgs // {
        inherit cargoArtifacts;
      };
    in
    {
      checks = {
        lean-browser-build = config.packages.lean-browser;

        # Clippy needs only the cargo sources.
        lean-browser-clippy = craneLib.cargoClippy (
          withDeps
          // {
            src = cargoSrc;
            cargoClippyExtraArgs = "--workspace --all-targets -- -D warnings";
          }
        );

        # Tests read the corpus/refs (full `src`) and shape text: the
        # sandbox has no system fonts, so point the renderer at DejaVu or
        # the text tests skip themselves and pass vacuously.
        lean-browser-test = craneLib.cargoTest (
          withDeps
          // {
            cargoTestExtraArgs = "--workspace";
            buildInputs = commonArgs.buildInputs ++ [ testFonts ];
            LEAN_FONT_DIR = "${testFonts}/share/fonts/truetype";
          }
        );

        lean-browser-fmt = craneLib.cargoFmt {
          inherit (commonArgs) pname version;
          src = cargoSrc;
        };
      };
    };
}
