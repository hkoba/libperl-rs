use strict;
use warnings;
use Test::More;

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
    eval { my @x = Mytest2::statfs("/no/such/path/here") };
    ok($@,                       'failure path croaked');
    like($@, qr/statvfs|No such/i, 'error message mentions the OS error');
}

# Vec<String> return: split words.
{
    my @w = Mytest2::words("the quick brown fox");
    is_deeply(\@w, [qw(the quick brown fox)], 'words splits on whitespace');

    my @empty = Mytest2::words("   ");
    is_deeply(\@empty, [], 'whitespace-only input is empty list');

    my @single = Mytest2::words("solo");
    is_deeply(\@single, ['solo'], 'single token');
}

done_testing;
