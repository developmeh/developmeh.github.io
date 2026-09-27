# The reusable flake-parts module. `leanInputs` are this flake's own locked
# inputs (nixpkgs, rust-overlay, crane), so an importing flake needs none of
# them itself:
#
#   imports = [ lean-browser.flakeModules.default ];
#
# The module adds `packages.lean-browser`, `devShells.lean-browser`, the
# `checks` and a formatter to the importing flake for every system it
# declares (x86_64-linux and aarch64-linux are the supported ones).
{ leanInputs }:
{ ... }:
{
  imports = [
    (import ./toolchain.nix { inherit leanInputs; })
    ./packages.nix
    ./devshell.nix
    ./checks.nix
    ./formatter.nix
  ];
}
