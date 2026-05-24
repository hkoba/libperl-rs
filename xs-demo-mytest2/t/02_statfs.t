use Test2::V0;

use Mytest2;

# Variable-length list return: perlxstut EXAMPLE 5 territory.
{
    my @a = Mytest2::statfs("/");
    is(scalar @a, 7, 'statfs("/") returns 7 values');
    ok($a[0] > 0,    'block size is positive');
    ok($a[2] >= $a[3], 'total blocks >= free blocks');
}

# Result<_, String> Err path → croak with the message.
{
    my $err = dies { my @x = Mytest2::statfs("/no/such/path/here") };
    ok($err,                         'failure path croaked');
    like($err, qr/statvfs|No such/i, 'error message mentions the OS error');
}

# Vec<String> return: split words.
{
    my @w = Mytest2::words("the quick brown fox");
    is(\@w, [qw(the quick brown fox)], 'words splits on whitespace');

    my @empty = Mytest2::words("   ");
    is(\@empty, [], 'whitespace-only input is empty list');

    my @single = Mytest2::words("solo");
    is(\@single, ['solo'], 'single token');
}

done_testing;
