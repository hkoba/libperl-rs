use strict;
use warnings;
use utf8;
use Test::More;

use Mytest2;

# foo(IV, IV, &CStr) -> NV  — perlxstut EXAMPLE 4 shape
is(Mytest2::foo(1,  2, "abc"),  6, '1 + 2 + length("abc") = 6');
is(Mytest2::foo(0,  0, ""),     0, 'all zeros = 0');
is(Mytest2::foo(10, 20, "hi"), 32, '10 + 20 + length("hi") = 32');

# Type returned is NV; even for whole-number results the SV is NV-flavored.
my $r = Mytest2::foo(1, 1, "a");
is($r, 3, 'foo returns 3 for (1,1,"a")');

# byte_len(&CStr) -> IV — string in, integer out
is(Mytest2::byte_len(""),       0, 'empty string is 0 bytes');
is(Mytest2::byte_len("hello"),  5, '"hello" is 5 bytes');

# shout(&str) -> String — UTF-8 in, UTF-8 out
is(Mytest2::shout("hello"), "HELLO",       'ASCII uppercased');
is(Mytest2::shout(""),      "",            'empty stays empty');
is(Mytest2::shout("HelloWorld"), "HELLOWORLD", 'mixed case → all caps');

# UTF-8 round-trip — ascii subset only for portability of this test file.
is(Mytest2::shout("rust"), "RUST", 'rust → RUST');

# Arity check still works after type extension.
eval { Mytest2::foo(1, 2) };
like($@, qr/Usage:/, 'wrong-arity foo croaks');

eval { Mytest2::shout() };
like($@, qr/Usage:/, 'no-arg shout croaks');

done_testing;
