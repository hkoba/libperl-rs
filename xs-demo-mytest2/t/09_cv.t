use Test2::V0;

use Mytest2;

# `Cv` argument kind — the caller passes a CODE reference, the body
# receives a Cv handle. Primary use case: static analysis of
# string-eval'd anonymous subs.
{
    my $anon = eval 'sub { 1 + 2 }';
    ok(!Mytest2::code_is_xsub($anon), 'eval anon sub is not an XSUB');
    like(Mytest2::code_file($anon), qr/eval/, 'CvFILE mentions eval');
    cmp_ok(Mytest2::code_op_count($anon), '>', 2, 'op chain non-trivial');
}

# XSUBs are visible as such and have no OP chain.
{
    ok(Mytest2::code_is_xsub(\&Mytest2::code_is_xsub), 'XS sub reports is_xsub');
    is(Mytest2::code_op_count(\&Mytest2::code_is_xsub), 0, 'XSUB has no op chain');
}

# Prototype access (CvPROTO composed from SvPOK + SvPVX_const/SvCUR).
{
    my $with_proto = eval 'sub ($$) { }';
    is(Mytest2::code_proto($with_proto), '$$', 'prototype string');
    my $anon = eval 'sub { }';
    is(Mytest2::code_proto($anon), '', 'no prototype -> empty string');
}

# Trampoline type check.
like(dies { Mytest2::code_file(42) }, qr/must be a CODE reference/,
    'non-ref croaks');
like(dies { Mytest2::code_file([]) }, qr/must be a CODE reference/,
    'non-code ref croaks');
like(dies { Mytest2::code_file() }, qr/Usage:/, 'arity croak');

done_testing;
