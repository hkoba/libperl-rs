#!/usr/bin/tclsh

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

#----------------------------------------

array set ::opts [cmdline::getoptions ::argv {
    {n "dry-run"}
    {q "quiet"}
    {step.arg "" "Run only specific steps"}
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

STEP 1 "バージョン bump" {
    RUN perl -i -s -ple {
        if (m{^\[package\]} ... m{^\[}) {
            s/^version = "(?:[^\"]+)"/version = "$newVersion"/
            and print STDERR "# Updated $ARGV: $_" and ++$ok;
        }
        END {exit 1 if not $ok}
    } -- -newVersion=$::newVersion \
        Cargo.toml libperl-sys/Cargo.toml libperl-macros/Cargo.toml
}

STEP 2 "それを手元 build 検証して commit" {
    RUN cargo build --workspace

    RUN cargo test --workspace

    RUN git commit -am "Bump libperl-sys / libperl-macros / libperl-rs to $::newVersion"
}


STEP 3 publish {
    RUN cargo publish -p libperl-sys -p libperl-macros -p libperl-rs
}

STEP 4 事後処理 {
    RUN git tag v$::newVersion -m "Bump version to v$::newVersion"

    RUN git push --tags && git push
}
