#!/bin/zsh

emulate -L zsh

set -e

realScriptFn=$(readlink -f $0)
topDir=$realScriptFn:h

SELINUX=""
if (($+commands[selinuxenabled])) && selinuxenabled; then
    SELINUX=":z"
fi

#========================================

zparseopts -D -K x=o_xtrace -image:=o_image

if (($#o_xtrace)); then set -x; fi

if (($#o_image)); then
    imageName=${o_image[2]#=}
else
    imageName=perl
fi

if ((! ARGC)); then
    argv=(--all)
fi

#========================================

exec docker run --rm -it -v $topDir:/app${SELINUX} $imageName /bin/bash -c '
 apt update &&
 apt install -y llvm-dev libclang-dev clang &&
 curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh &&
 sh rustup.sh -y &&
 source $HOME/.cargo/env &&
 cd /app &&
 cargo test '"$*"
