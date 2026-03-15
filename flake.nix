{
  description = "Developmeh.com Env";

inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/25.11";
    deploy.url = "path:./deploy";
    kwik-e-mart.url = "sourcehut:~ninjapanzer/kwik-e-mart";
  };

  outputs = { self, nixpkgs, kwik-e-mart, deploy }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSupportedSystem = f: nixpkgs.lib.genAttrs supportedSystems (system: f {
        pkgs = import nixpkgs { inherit system; };
        kwike = kwik-e-mart.packages.${system}.default;
        deployShell = deploy.outputs.devShells.${system}.default;
      });
    in
    {
      devShells = forEachSupportedSystem ({ pkgs, kwike, deployShell }: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            marksman
            typos-lsp
            dasel  # TOML/JSON/YAML manipulation for discussion sync
            jq     # JSON parsing for GitHub GraphQL API
            kwike
          ];

          inputsFrom = [
            deployShell
          ];
        };
      });
    };
}
