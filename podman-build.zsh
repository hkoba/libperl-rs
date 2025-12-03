#!/bin/zsh

realScriptFn=$(readlink -f $0)
appDir=$realScriptFn:h

podman run --rm -it \
  -v "$appDir":/work:O -w /work \
  rust:1.89-bookworm \
  bash -lc '
    apt-get update &&
    apt-get install -y \
      build-essential pkg-config \
      perl libperl-dev \
      llvm-dev libclang-dev clang &&
    /usr/local/cargo/bin/rustc --version &&
    perl -V:version &&
    RUSTFLAGS="-D warnings" /usr/local/cargo/bin/cargo build &&
    RUSTFLAGS="-D warnings" /usr/local/cargo/bin/cargo test --all --examples
  '
