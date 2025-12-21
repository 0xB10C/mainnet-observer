{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {

  shellHook = ''
    # during the integration tests, don't try to download a bitcoind binary
    # use the nix one instead
    export BITCOIND_SKIP_DOWNLOAD=1
    export BITCOIND_EXE=${pkgs.bitcoind}/bin/bitcoind
  '';

  nativeBuildInputs = [
    # backend
    pkgs.cargo
    pkgs.rustc
    pkgs.rustfmt
    pkgs.clippy
    pkgs.sqlite
    pkgs.diesel-cli

    # for integration-tests
    pkgs.bitcoind
    pkgs.bzip2.dev

    # frontend
    pkgs.hugo
    pkgs.optipng
  ];
}
