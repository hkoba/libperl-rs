use Test2::V0;

use Mytest2;

# Phase 3.10c: `Av` / `Hv` newtypes + `Rv<T>` reference returns.

# `make_pair` returns an array reference.
{
    my $r = Mytest2::make_pair();
    is(ref($r), 'ARRAY', 'make_pair: returns array ref');
    is($r, [1, 2], 'make_pair: contents are [1, 2]');
}

# Each call gets a fresh array (no aliasing).
{
    my $r1 = Mytest2::make_pair();
    my $r2 = Mytest2::make_pair();
    # Test2::V0's `isnt` is structural; we want pointer inequality, so
    # stringify the refs (`ARRAY(0x...)`) and compare those.
    isnt("$r1", "$r2", 'make_pair: distinct refs across calls');
    push @$r1, 99;
    is($r2, [1, 2], 'make_pair: mutating r1 does not touch r2');
}

# `make_record` returns a hash reference.
{
    my $r = Mytest2::make_record();
    is($r, hash {
        field name => 'ada';
        field year => 1815;
        end;
    }, 'make_record: { name => "ada", year => 1815 }');
}

# `Option<Rv<Av>>` — Some / None branches.
is(Mytest2::maybe_pair(1), [10, 20], 'maybe_pair(1): Some -> arrayref');
is(Mytest2::maybe_pair(0), U(),      'maybe_pair(0): None -> undef');

# Refcount sanity: 1000 round-trips of each constructor should not
# leak memory or abort.
{
    Mytest2::make_pair()   for 1..1000;
    Mytest2::make_record() for 1..1000;
    Mytest2::maybe_pair($_ % 2) for 1..1000;
    pass('1000 ctor calls each ran without abort');
}

# Arity checks (context arg must NOT consume a slot).
like(dies { Mytest2::make_pair(99) }, qr/Usage:/, 'make_pair with extra arg croaks');
like(dies { Mytest2::maybe_pair() },  qr/Usage:/, 'maybe_pair with no arg croaks');

done_testing;
