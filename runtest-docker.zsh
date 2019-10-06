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

zparseopts -D -K x=o_xtrace n=o_dryrun \
           -image:=o_image -target:=o_targetDir

if (($#o_xtrace)); then set -x; fi

function x {
    print -r -- '#' ${(@q-)argv}
    if (($#o_dryrun)); then return; fi
    "$@"
}

if (($#o_image)); then
    imageName=${o_image[2]#=}
else
    imageName=perl
fi

volOpts=()
if (($#o_targetDir)); then
    volOpts+=(-v ${o_targetDir[2]#=}:/app/target${SELINUX})
fi

if ((! ARGC)); then
    argv=(--all)
fi

#========================================

cmdList=(
    'apt update'
    'apt install -y llvm-dev libclang-dev clang'
    'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh'
    'sh rustup.sh -y'
    'source $HOME/.cargo/env'
    'rustup component add rustfmt'
    'cd /app'
)

if ((! $#o_targetDir)); then
    cmdList+=('cargo clean')
fi

cmdList+=(
    'cargo build -vv'
)

if ((ARGC)); then
    cmdList+=("${(j/ /)${(@q-)argv}}")
else
    cmdList+=("cargo test --all")
fi


x exec docker run --rm -it \
     -v $topDir:/app${SELINUX} \
     $volOpts \
     $imageName /bin/bash -c "${(j/&&/)cmdList}"
