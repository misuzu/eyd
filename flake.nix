{
  outputs =
    inputs:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      sources = import ./npins;
      forAllSystems =
        f:
        builtins.listToAttrs (
          map (system: {
            name = system;
            value = f system;
          }) systems
        );
      pkgsFor =
        system:
        import sources.nixpkgs {
          inherit system;
          overlays = [ inputs.self.overlays.default ];
        };
    in
    {
      overlays.default = final: prev: {
        eyd = final.callPackage ./package.nix { };
      };
      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            inputsFrom = [ pkgs.eyd ];
            packages = [ pkgs.npins ];
          };
        }
      );
      packages = forAllSystems (system: {
        default = (pkgsFor system).eyd;
      });
      nixosModules = {
        eyd = ./nixos-module.nix;
        default = {
          imports = [ inputs.self.nixosModules.eyd ];
          nixpkgs.overlays = [ inputs.self.overlays.default ];
        };
      };
      checks = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          nixos-module = pkgs.testers.runNixOSTest ./nixos-test.nix;
        }
      );
    };
}
