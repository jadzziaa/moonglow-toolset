{
  description = "Moonglow Toolset: a module toolset for Neverwinter Nights: Enhanced Edition";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "moonglow";
          version = (lib.importTOML ./Cargo.toml).workspace.package.version;

          src = self;
          # Read from the repository, so a release needs no hash updated here.
          cargoLock.lockFile = ./Cargo.lock;

          cargoBuildFlags = [
            "-p"
            "moonglow"
            "-p"
            "mg"
          ];
          # The tests need the game install and a GPU, which the build sandbox
          # doesn't have.
          doCheck = false;

          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = [ pkgs.alsa-lib ];

          # wgpu and winit open these at run time instead of linking them.
          postFixup = ''
            wrapProgram $out/bin/moonglow \
              --prefix LD_LIBRARY_PATH : ${
                lib.makeLibraryPath [
                  pkgs.libGL
                  pkgs.libxkbcommon
                  pkgs.vulkan-loader
                  pkgs.wayland
                  pkgs.libx11
                  pkgs.libxcursor
                  pkgs.libxi
                  pkgs.libxrandr
                ]
              }
          '';

          postInstall = ''
            id=io.github.moonglow_toolset.Moonglow
            install -Dm644 packaging/linux/$id.desktop -t $out/share/applications
            install -Dm644 packaging/linux/moonglow-mime.xml $out/share/mime/packages/$id.xml
            install -Dm644 packaging/icons/moonglow.svg $out/share/icons/hicolor/scalable/apps/$id.svg
            for s in 16 24 32 48 64 128 256 512; do
              install -Dm644 packaging/icons/moonglow-$s.png $out/share/icons/hicolor/''${s}x$s/apps/$id.png
            done
          '';

          meta = {
            description = "A module toolset for Neverwinter Nights: Enhanced Edition";
            homepage = "https://github.com/jadzziaa/moonglow-toolset";
            license = lib.licenses.gpl3Only;
            platforms = systems;
            mainProgram = "moonglow";
          };
        };
      });
    };
}
