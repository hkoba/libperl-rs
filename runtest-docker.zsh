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
           -reuse-target-dir=o_reuse_target_dir \
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
reuse_target_dir=0
if (($#o_targetDir)); then
    reuse_target_dir=0
    volOpts+=(-v ${o_targetDir[2]#=}:/app/target${SELINUX})
elif ((! $#o_reuse_target_dir)); then
    reuse_target_dir=0
    dn=$topDir.target.${imageName:gs/:/__/}
    mkdir -vp $dn
    volOpts+=(-v $dn:/app/target${SELINUX})
else
    reuse_target_dir=1
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

if (($reuse_target_dir)); then
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
