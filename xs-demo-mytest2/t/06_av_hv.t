use strict;
use warnings;
use utf8;
use Test::More;

use Mytest2;

# Phase 3.10c: `Av` / `Hv` newtypes + `Rv<T>` reference returns.

# `make_pair` returns an array reference.
{
    my $r = Mytest2::make_pair();
    is(ref($r), 'ARRAY', 'make_pair: returns array ref');
    is_deeply($r, [1, 2], 'make_pair: contents are [1, 2]');
}

# Each call gets a fresh array (no aliasing).
{
    my $r1 = Mytest2::make_pair();
    my $r2 = Mytest2::make_pair();
    isnt($r1, $r2, 'make_pair: distinct refs across calls');
    push @$r1, 99;
    is_deeply($r2, [1, 2], 'make_pair: mutating r1 does not touch r2');
}

# `make_record` returns a hash reference.
{
    my $r = Mytest2::make_record();
    is(ref($r), 'HASH', 'make_record: returns hash ref');
    is($r->{name}, 'ada',  'make_record: name is "ada"');
    is($r->{year}, 1815,   'make_record: year is 1815');
    is_deeply([sort keys %$r], [qw(name year)], 'make_record: exactly two keys');
}

# `Option<Rv<Av>>` — Some / None branches.
{
    my $r = Mytest2::maybe_pair(1);
    is_deeply($r, [10, 20], 'maybe_pair(1): Some -> arrayref');
}
{
    my $r = Mytest2::maybe_pair(0);
    ok(!defined($r), 'maybe_pair(0): None -> undef');
}

# Refcount sanity: 1000 round-trips of each constructor should not
# leak memory or abort.
{
    Mytest2::make_pair()   for 1..1000;
    Mytest2::make_record() for 1..1000;
    Mytest2::maybe_pair($_ % 2) for 1..1000;
    pass('1000 ctor calls each ran without abort');
}

# Arity checks (context arg must NOT consume a slot).
eval { Mytest2::make_pair(99) };
like($@, qr/Usage:/, 'make_pair with extra arg croaks');

eval { Mytest2::maybe_pair() };
like($@, qr/Usage:/, 'maybe_pair with no arg croaks');

done_testing;
