{
  description = "Environment setup tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/15f4ee454b1dce334612fa6843b3e05cf546efab";
  };

  outputs = { nixpkgs, ... }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system: {
        default = nixpkgs.legacyPackages.${system}.gh;
        codex = nixpkgs.legacyPackages.${system}.codex;
        gh = nixpkgs.legacyPackages.${system}.gh;
        neovim = nixpkgs.legacyPackages.${system}.neovim;
        tmux = nixpkgs.legacyPackages.${system}.tmux;
      });

      devShells = forAllSystems (system: {
        default = nixpkgs.legacyPackages.${system}.mkShell {
          packages = [
            nixpkgs.legacyPackages.${system}.gh
          ];
        };
      });
    };
}
