use Test2::V0;

use Mytest2;

# Phase 3.10e: end-to-end demo. perlxstut EXAMPLE 6 territory.
#
# `multi_statfs(\@paths)` returns a hashref keyed by path. Each value
# is either a 7-element arrayref (statvfs results) or a string error
# message.

# Happy path: every path is statvfs-able.
{
    my $r = Mytest2::multi_statfs(['/', '/tmp']);
    is(ref($r), 'HASH', 'returns hashref');
    is([sort keys %$r], ['/', '/tmp'], 'one entry per input path');

    is(ref($r->{'/'}),    'ARRAY', '"/" entry is arrayref (success)');
    is(ref($r->{'/tmp'}), 'ARRAY', '"/tmp" entry is arrayref (success)');
    is(scalar @{$r->{'/'}},    7, '"/" has 7 statvfs fields');
    is(scalar @{$r->{'/tmp'}}, 7, '"/tmp" has 7 statvfs fields');

    ok($r->{'/'}->[0] > 0,            'block size positive');
    ok($r->{'/'}->[2] >= $r->{'/'}->[3], 'total blocks >= free blocks');
}

# Mixed: one good + one bad path. Bad gets an error string, good
# still gets a 7-arrayref.
{
    my $r = Mytest2::multi_statfs(['/tmp', '/no/such/dir/xyz123']);
    is(ref($r->{'/tmp'}),                'ARRAY', '/tmp succeeds');
    is(ref($r->{'/no/such/dir/xyz123'}), '',      '/no/such/dir is plain scalar (error msg)');
    like($r->{'/no/such/dir/xyz123'},
         qr/statvfs failed:/,
         '... and the message starts with "statvfs failed:"');
}

# Empty input → empty hash.
is(Mytest2::multi_statfs([]), {}, 'empty paths → empty hash');

# Single path.
{
    my $r = Mytest2::multi_statfs(['/']);
    is([keys %$r], ['/'],         'single path → single key');
    is(ref($r->{'/'}), 'ARRAY',   '... with arrayref value');
}

# Type-mismatch on outer arg: should croak with the proc-macro's
# uniform error message.
like(dies { Mytest2::multi_statfs("not an arrayref") },
     qr/must be a ARRAY reference/,
     'non-arrayref input croaks');

like(dies { Mytest2::multi_statfs({}) },
     qr/must be a ARRAY reference/,
     'hashref input croaks');

# Arity errors.
like(dies { Mytest2::multi_statfs() },           qr/Usage:/, 'no args croaks');
like(dies { Mytest2::multi_statfs(['/'], 'x') }, qr/Usage:/, 'extra args croaks');

# Smoke / leak: 500 iterations with mixed good/bad paths.
{
    Mytest2::multi_statfs(['/', '/tmp', '/no/such/path']) for 1..500;
    pass('500 multi_statfs calls ran without abort');
}

done_testing;
