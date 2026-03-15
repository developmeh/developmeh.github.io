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

          shellHook = ''
            # Kwike publish workflow scripts
            export PATH="$PWD/.kwike/scripts:$PATH"

            # Convenience aliases
            alias kwike-start=".kwike/scripts/start-kwike.sh start"
            alias kwike-stop=".kwike/scripts/start-kwike.sh stop"
            alias kwike-status=".kwike/scripts/start-kwike.sh status"
            alias kwike-setup=".kwike/scripts/setup-hooks.sh"

            echo ""
            echo "Kwike publish workflow available:"
            echo "  kwike-setup   - Install git hooks (run once)"
            echo "  kwike-start   - Start daemon and consumers"
            echo "  kwike-stop    - Stop all kwike processes"
            echo "  kwike-status  - Check kwike status"
            echo ""
          '';
        };
      });
    };
}
