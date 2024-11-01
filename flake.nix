{
  description = "the lunatix kernel project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { system = system; };
      crossPkgs = import nixpkgs {
        localSystem = system;
        crossSystem = { config = "riscv64-linux-gnu"; };
      };
      #rustToolchain = (builtins.fromTOML (builtins.readFile "rust-toolchain.toml")).toolchain;
      #cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
    in
    {
      devShells.default = pkgs.mkShell rec {
        LD_LIBRARY_PATH = with pkgs.pkgsStatic; lib.makeLibraryPath [ openssl openssl.dev ];
        C_INCLUDE_PATH = with pkgs.pkgsStatic; lib.makeIncludePath [ openssl openssl.dev ];
        CPLUS_INCLUDE_PATH = C_INCLUDE_PATH;
        PKG_CONFIG_PATH = with pkgs.pkgsStatic; lib.makeSearchPathOutput "dev" "lib/pkgconfig" [ openssl ];
        packages = [
          pkgs.gnumake
          pkgs.rustup
          pkgs.pre-commit
          pkgs.bison
          pkgs.flex
          crossPkgs.buildPackages.gcc
        ];
      };
    });
}
