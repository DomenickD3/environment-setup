{
  description = "Environment setup tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/15f4ee454b1dce334612fa6843b3e05cf546efab";
    nixpkgs-codex.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { nixpkgs, nixpkgs-codex, ... }:
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
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          codexPkgs = nixpkgs-codex.legacyPackages.${system};
        in
        {
          default = pkgs.gh;
          codex = codexPkgs.codex;
          gh = pkgs.gh;
          neovim = pkgs.neovim;
          stow = pkgs.stow;
          tmux = pkgs.tmux;
        }
      );

      devShells = forAllSystems (system: {
        default = nixpkgs.legacyPackages.${system}.mkShell {
          packages = [
            nixpkgs.legacyPackages.${system}.gh
          ];
        };
      });
    };
}
