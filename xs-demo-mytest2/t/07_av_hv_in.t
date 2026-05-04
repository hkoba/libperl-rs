use Test2::V0;

use Mytest2;

# Phase 3.10d: `&Av` / `&Hv` borrowed-reference arguments.

# `sum_iv(\@arr)` — sum integer elements via `Av::iter`.
{
    is(Mytest2::sum_iv([1, 2, 3]),       6,  'sum_iv: [1,2,3] = 6');
    is(Mytest2::sum_iv([]),              0,  'sum_iv: empty = 0');
    is(Mytest2::sum_iv([10, -3, 100]), 107,  'sum_iv: mixed sign');
    is(Mytest2::sum_iv([42]),           42,  'sum_iv: single element');

    # Sparse array: undef slots contribute nothing (Av::iter yields
    # `Option<Sv>` and we filter out `None`).
    my @sparse;
    $sparse[0] = 5;
    $sparse[3] = 7;
    is(Mytest2::sum_iv(\@sparse), 12, 'sum_iv: sparse [5,,,7] = 12');
}

# `av_len_demo(\@arr)` — `Av::len`.
is(Mytest2::av_len_demo([1, 2, 3, 4]), 4, 'av_len_demo: 4 elements');
is(Mytest2::av_len_demo([]),           0, 'av_len_demo: empty');

# `record_keys(\%hash)` — sorted keys via `Hv::iter`. The sub returns
# a `Vec<String>`, which the proc-macro pushes as a flat list — wrap
# with `[ ... ]` to compare against the expected arrayref.
{
    is([Mytest2::record_keys({ a => 1, b => 2, c => 3 })],
       ['a', 'b', 'c'],
       'record_keys: sorted keys');
    is([Mytest2::record_keys({})], [], 'record_keys: empty hash');
    is([Mytest2::record_keys({ only => 1 })], ['only'], 'record_keys: single key');
}

# Round-trip with the existing `make_record` helper from 3.10c.
{
    my $r = Mytest2::make_record();
    is([Mytest2::record_keys($r)], ['name', 'year'], 'record_keys on make_record output');
}

# Type-mismatch errors: ROK + SVt check should croak with a readable
# message before the body fn ever runs.
like(dies { Mytest2::sum_iv("not an arrayref") },
     qr/must be a ARRAY reference/,
     'sum_iv on non-ref croaks');
like(dies { Mytest2::sum_iv({ a => 1 }) },
     qr/must be a ARRAY reference/,
     'sum_iv on hashref croaks');
like(dies { Mytest2::record_keys([1, 2, 3]) },
     qr/must be a HASH reference/,
     'record_keys on arrayref croaks');
like(dies { Mytest2::record_keys(undef) },
     qr/must be a HASH reference/,
     'record_keys on undef croaks');

# Arity checks (context arg consumes no slot).
like(dies { Mytest2::sum_iv() },           qr/Usage:/, 'sum_iv with no args croaks');
like(dies { Mytest2::sum_iv([1], "extra") }, qr/Usage:/, 'sum_iv with extra arg croaks');

# Leak smoke: 1000 iterations.
{
    Mytest2::sum_iv([1, 2, 3, 4, 5]) for 1..1000;
    Mytest2::record_keys({ a => 1, b => 2, c => 3 }) for 1..1000;
    pass('1000 iterations of each ran without abort');
}

done_testing;
