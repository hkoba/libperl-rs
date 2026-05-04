use Test2::V0;

use Mytest;

ok( Mytest::is_even(4),   '4 is even');
ok( Mytest::is_even(0),   '0 is even');
ok( Mytest::is_even(-2),  '-2 is even');

ok(!Mytest::is_even(3),   '3 is odd');
ok(!Mytest::is_even(-1),  '-1 is odd');
ok(!Mytest::is_even(99),  '99 is odd');

# Arity check: calling with wrong number of args should croak.
like(dies { Mytest::is_even() },     qr/Usage:/, 'no-arg call croaks with Usage:');
like(dies { Mytest::is_even(1, 2) }, qr/Usage:/, 'two-arg call croaks with Usage:');

done_testing;
