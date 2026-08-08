{
  description = "Developmeh.com Env";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/25.11";

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSupportedSystem = f: nixpkgs.lib.genAttrs supportedSystems (system: f {
        pkgs = import nixpkgs { inherit system; };
      });
    in
    {
      devShells = forEachSupportedSystem ({ pkgs }: {
        default = pkgs.mkShell {
          # zola builds the site; bats/jq/python3 run tests/discoverability.bats
          # against the build output in CI. Same pinned nixpkgs as the build, so
          # local and CI agree.
          packages = with pkgs; [ zola bats jq python3 ];
        };
      });
    };
}
