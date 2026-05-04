use Test2::V0;

use Mytest2;

# `Sv` newtype passthrough — should behave identically to the raw
# `*mut SV` version of identity.
{
    is(Mytest2::identity_sv(42),       42,       'IV passthrough via Sv');
    is(Mytest2::identity_sv("hello"),  "hello",  'PV passthrough via Sv');
    is(Mytest2::identity_sv(3.14),     3.14,     'NV passthrough via Sv');
}

# `Option<Sv>` — Some / None branches.
{
    is(Mytest2::maybe_sv2("kept", 1),    "kept", 'Some(Sv) returns input');
    is(Mytest2::maybe_sv2("dropped", 0), U(),    'None returns undef');
}

is(Mytest2::maybe_sv2(123, 0), U(), 'maybe_sv2(_, 0) is undef');

# Refcount: 1000 round-trips should not leak / abort.
{
    my $x = "leak-check";
    Mytest2::identity_sv($x) for 1..1000;
    pass('1000 Sv round-trips ran without abort');
}

# Arity check still works for Sv-typed args.
like(dies { Mytest2::identity_sv() }, qr/Usage:/, 'no-arg identity_sv croaks');
like(dies { Mytest2::maybe_sv2(1) },  qr/Usage:/, 'wrong-arity maybe_sv2 croaks');

done_testing;
