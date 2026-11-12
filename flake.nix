{
  outputs =
    inputs:
    let
      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems =
        f:
        builtins.listToAttrs (
          map (system: {
            name = system;
            value = f system;
          }) systems
        );
      forAllSystemsPkgs =
        f:
        forAllSystems (
          system:
          let
            sources = import ./npins;
            nixpkgs = import sources.nixpkgs { inherit system; };
            inherit (nixpkgs) pkgs;
          in
          f pkgs
        );
    in
    {
      nixosModules.default = ./nixos-module.nix;
      checks = forAllSystemsPkgs (pkgs: {
        eyd = pkgs.testers.runNixOSTest ./nixos-test.nix;
      });
      packages = forAllSystemsPkgs (pkgs: {
        eyd = pkgs.callPackage ./eyd/package.nix { };
      });
    };
}
