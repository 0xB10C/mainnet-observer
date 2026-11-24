{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {

  shellHook = ''
    # during the integration tests, don't try to download a bitcoind binary
    # use the nix one instead
    export BITCOIND_SKIP_DOWNLOAD=1
    export BITCOIND_EXE=${pkgs.bitcoind}/bin/bitcoind

    # cargo-llvm-cov looks for the llvm-tools-preview rustup component, which
    # doesn't exist here. Point it at the nixpkgs LLVM binaries instead. These
    # need to match the LLVM version rustc was built with.
    export LLVM_COV=${pkgs.llvmPackages.llvm}/bin/llvm-cov
    export LLVM_PROFDATA=${pkgs.llvmPackages.llvm}/bin/llvm-profdata
  '';

  nativeBuildInputs = [
    # backend
    pkgs.cargo
    pkgs.rustc
    pkgs.rustfmt
    pkgs.clippy
    pkgs.sqlite
    pkgs.diesel-cli

    # for test coverage and mutation testing
    pkgs.cargo-mutants
    pkgs.cargo-llvm-cov
    pkgs.llvmPackages.llvm # provides llvm-cov and llvm-profdata

    # for integration-tests
    pkgs.bitcoind
    pkgs.bzip2.dev

    # frontend
    pkgs.hugo
    pkgs.optipng
    pkgs.zopfli
    pkgs.pngquant
  ];
}
