# `nix fmt` formats the Nix files; Rust is formatted by rustfmt (see
# checks.nix).
{ ... }:
{
  perSystem =
    { config, ... }:
    {
      formatter = config.lean.pkgs.nixfmt-rfc-style;
    };
}
