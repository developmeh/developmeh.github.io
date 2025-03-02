{
  description = "Developmeh.com Env";

inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    deploy.url = "path:./deploy";
  };

  outputs = { self, nixpkgs, deploy }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSupportedSystem = f: nixpkgs.lib.genAttrs supportedSystems (system: f {
        pkgs = import nixpkgs { inherit system; };
        deployShell = deploy.outputs.devShells.${system}.default;
      });
    in
    {
      devShells = forEachSupportedSystem ({ pkgs, deployShell }: {
        default = pkgs.mkShell {
          packages = with pkgs; [ marksman typos-lsp ];

          inputsFrom = [
            deployShell
          ];
        };
      });
    };
}
