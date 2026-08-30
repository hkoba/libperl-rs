#!/usr/bin/tclsh
# publish.tcl — libperl-rs workspace のリリース手順
# (事前検証・準備ステップは libperl-macrogen/publish.tcl から逆移植)
#
# usage: ./publish.tcl [-n] [-step {1 2 ...}] [-docker LEGS] [-yes]
#
# 本 workspace 固有の注意点をステップ化している:
#
# 1. 前提チェック (STEP 1): clean tree / master ブランチ / git pull に加え、
#    [patch.crates-io] が残っていないこと (multi-perl ハーネスは scratch
#    copy にしか注入しないが、手動実験の残骸があると検証が working tree の
#    macrogen 相手になってしまう)。libperl-macrogen の pin より新しい版が
#    crates.io に出ていれば警告する (bump 忘れ防止)
# 2. publish 前のローカル検証 (STEP 2): bump 前に workspace 全体の
#    build + test (--examples 込み = CI と同等) を回す
# 3. multi-perl docker 検証 (STEP 3): runtest-docker.zsh で代表 perl
#    イメージに対して build + test。EOL Debian の版 (<=5.26 系) を足す
#    ときは --use-debian-archive が要るので default には入れない
# 4. パッケージ内容の検査 (STEP 6): libperl-sys の build.rs が利用者側
#    ビルド時に読むファイル (wrapper.h / require-codegen*.txt /
#    skip-codegen*.txt) の同梱と、root の dev ファイル除外を確認する

#----------------------------------------
# my librun
package require cmdline

proc RUN args {
    puts "# $args"
    if {$::opts(n)} return
    # リダイレクトが指定されていない時は stdout へリダイレクト。末尾のみ認識。
    if {[lindex $args end-1] ni {">" ">@" ">>"}} {
        lappend args >@ stdout
    }
    =RUN {*}$args
}

proc =RUN args {
    exec -ignorestderr {*}$args 2>@ stderr
}

proc o_dryrun {} {
    if {$::opts(n)} {list -n}
}

# Tcl 自体のコマンドを dry-run にしたいときは ** を使う
proc ** args {
    puts "# $args"
    if {$::opts(n)} return
    {*}$args
}
#----------------------------------------

proc readPackageVersion {tomlFn} {
    =RUN perl -nle {
        next unless m{^\[package\]} ... m{^\[};
        /^version = "([^\"]+)"/ and print $1 and ++$ok;
        END {exit 1 if not $ok}
    } $tomlFn
}

proc incrementMinorVersion verStr {
    set verList [split $verStr .]
    set minor [lindex $verList end]
    set newVerList [lreplace $verList end end [incr minor]]
    join $newVerList .
}

proc confirm {msg} {
    if {$::opts(yes) || $::opts(n)} { return 1 }
    puts -nonewline "$msg \[y/N] "
    flush stdout
    if {[gets stdin ans] < 0} { return 0 }
    string match -nocase y* [string trim $ans]
}

#----------------------------------------

array set ::opts [cmdline::getoptions ::argv {
    {n "dry-run"}
    {q "quiet"}
    {step.arg "" "Run only specific steps"}
    {docker.arg "5.32-threaded 5.44" "docker 検証の perl イメージタグ (空で skip)"}
    {yes "publish 前の確認プロンプトを省略"}
}]

proc STEP {n message command} {
    if {$::opts(step) eq ""} {
        puts "# ($n) $message"
    } elseif {$n ni $::opts(step)} {
        return;
    }
    uplevel #0 $command
}

#----------------------------------------

cd [file dirname [file normalize [info script]]]

set ::currentVersion [readPackageVersion Cargo.toml]
set ::newVersion     [incrementMinorVersion $::currentVersion]
set ::lastTag        [=RUN git tag -l v* --sort=-v:refname | head -1]

puts "# libperl-rs workspace $::currentVersion -> $::newVersion (前回 tag: $::lastTag)"

STEP 1 "前提チェック (clean tree / master / patch 残骸 / macrogen pin)" {
    if {[=RUN git status --porcelain --untracked-files=no] ne ""} {
        error "working tree が clean ではありません (commit してから実行して下さい)"
    }
    if {[=RUN git branch --show-current] ne "master"} {
        error "master ブランチではありません"
    }

    RUN git pull

    # [patch.crates-io] の残骸チェック — あると STEP 2/3 の検証が
    # crates.io 版でなくローカル macrogen 相手になってしまう
    if {![catch {=RUN grep -n {^\[patch\.crates-io\]} Cargo.toml} hit]} {
        error "Cargo.toml に \[patch.crates-io] が残っています: $hit"
    }

    # libperl-sys が pin する libperl-macrogen の版を表示し、crates.io の
    # 最新版より古ければ警告 (ネットワーク不調時は警告のみで続行)
    set pin ""
    regexp {libperl-macrogen = "([^"]+)"} \
        [=RUN cat libperl-sys/Cargo.toml] -> pin
    puts "# libperl-sys の libperl-macrogen pin: \"$pin\""
    if {[catch {
        set latest ""
        regexp {libperl-macrogen = "([^"]+)"} \
            [=RUN cargo search libperl-macrogen --limit 1] -> latest
        if {$latest ne "" && $latest ne $pin} {
            puts "# WARN: crates.io の最新 libperl-macrogen は $latest です\
 (pin: $pin — bump 忘れでないか確認して下さい)"
        }
    } err]} {
        puts "# WARN: crates.io の版確認をスキップ ($err)"
    }
}

STEP 2 "ローカル検証 (bump 前に workspace build + test --examples)" {
    set ambient [=RUN perl -MConfig -le {print "$Config{version} $Config{usethreads}"}]
    puts "# ambient perl: $ambient"
    RUN cargo build --workspace
    RUN cargo test --workspace --examples
}

STEP 3 "multi-perl docker 検証 (runtest-docker.zsh)" {
    if {$::opts(docker) eq ""} {
        puts "# -docker \"\" のため skip"
    } else {
        foreach leg $::opts(docker) {
            RUN ./runtest-docker.zsh --image perl:$leg
        }
    }
}

STEP 4 "バージョン bump (libperl-rs / libperl-sys / libperl-macros)" {
    RUN perl -i -s -ple {
        if (m{^\[package\]} ... m{^\[}) {
            s/^version = "(?:[^\"]+)"/version = "$newVersion"/
            and print STDERR "# Updated $ARGV: $_" and ++$ok;
        }
        END {exit 1 if not $ok}
    } -- -newVersion=$::newVersion \
        Cargo.toml libperl-sys/Cargo.toml libperl-macros/Cargo.toml
}

STEP 5 "bump 後の build 検証 + commit" {
    RUN cargo build --workspace
    RUN git commit -am "Bump libperl-sys / libperl-macros / libperl-rs to $::newVersion"
}

STEP 6 "パッケージ内容の検査 (build.rs 入力の同梱 / dev ファイル除外)" {
    # dry-run では STEP 5 の commit が行われないため dirty を許容する
    set extra [expr {$::opts(n) ? {--allow-dirty} : {}}]

    # libperl-sys: 利用者側ビルドで build.rs が読むファイルが必須
    set files [split [=RUN cargo package --list -p libperl-sys {*}$extra] \n]
    foreach need {
        wrapper.h xs-wrapper.h
        require-codegen.txt require-codegen-threaded.txt
        skip-codegen.txt skip-codegen-legacy.txt skip-codegen-pre42.txt
    } {
        if {$need ni $files} {
            error "libperl-sys の cargo package に $need が含まれていません"
        }
    }
    puts "# libperl-sys package OK ([llength $files] files)"

    # root (libperl-rs): dev 用ファイルが exclude されていること
    set files [split [=RUN cargo package --list -p libperl-rs {*}$extra] \n]
    foreach banned {CLAUDE.md runtest-docker.zsh podman-build.zsh publish.tcl} {
        if {$banned in $files} {
            error "libperl-rs の cargo package に $banned が含まれています (exclude 設定を確認)"
        }
    }
    foreach f $files {
        if {[string match docs/* $f]} {
            error "libperl-rs の cargo package に docs/ 配下が含まれています: $f"
        }
    }
    puts "# libperl-rs package OK ([llength $files] files)"
}

STEP 7 publish {
    if {![confirm "crates.io へ libperl-sys / libperl-macros / libperl-rs\
 $::newVersion を publish します。よろしいですか?"]} {
        error "中止しました"
    }
    RUN cargo publish -p libperl-sys -p libperl-macros -p libperl-rs
}

STEP 8 "事後処理 (tag + push)" {
    RUN git tag v$::newVersion -m "Bump version to v$::newVersion"

    # 注: Tcl の exec は `&&` をシェル演算子として解釈しないため 2 コマンドに分ける
    RUN git push --tags
    RUN git push

    puts "# 次の手作業:"
    puts "#   - PartialEval 側 CI (workflow_dispatch, input libperl-rs-ref) で probe 確認"
    puts "#   - docs.rs のビルド結果確認 (libperl-sys / libperl-rs)"
}
