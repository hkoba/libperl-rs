use strict;
use warnings;
use Test::More;

use Mytest2;

# identity($sv) — should round-trip any scalar.
{
    is(Mytest2::identity(42),       42,       'IV passthrough');
    is(Mytest2::identity("hello"),  "hello",  'PV passthrough');
    is(Mytest2::identity(3.14),     3.14,     'NV passthrough');
    is(Mytest2::identity(undef),    undef,    'undef passthrough');
}

# Refcount: identity should not leak. The arg's refcount stays 1
# (only @_ holds it). After return, the call expression should not
# leave the SV around.
{
    my $x = "leak-check";
    my $rc = Mytest2::identity($x);
    is($rc, "leak-check", 'identity returns same value');
    # If passthrough leaked an INC, $x's refcount would drift, but
    # there's no clean way to read it from pure Perl. We just check
    # that nothing crashes / asserts under repeated use.
    Mytest2::identity($x) for 1..1000;
    pass('1000 round-trips ran without abort');
}

# maybe_sv: Some / None paths.
{
    is(Mytest2::maybe_sv("kept", 1), "kept", 'Some branch returns input');
    is(Mytest2::maybe_sv("dropped", 0), undef, 'None branch returns undef');
}

# undef returned from None should be the actual undef, not 0/"".
{
    my $r = Mytest2::maybe_sv(123, 0);
    ok(!defined($r), 'maybe_sv(_, 0) is undef');
}

# Arity check still works.
eval { Mytest2::identity() };
like($@, qr/Usage:/, 'no-arg identity croaks');

eval { Mytest2::maybe_sv(1) };
like($@, qr/Usage:/, 'wrong-arity maybe_sv croaks');

done_testing;
