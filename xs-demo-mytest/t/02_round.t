use strict;
use warnings;
use Test::More;

use Mytest;

# Round positive halfway values up.
{
    my $x = 4.5;
    Mytest::round($x);
    is($x, 5,  '4.5 rounds to 5');
}
{
    my $x = 4.1;
    Mytest::round($x);
    is($x, 4,  '4.1 rounds to 4');
}

# Round negative halfway values away from zero.
{
    my $x = -4.5;
    Mytest::round($x);
    is($x, -5, '-4.5 rounds to -5');
}
{
    my $x = -4.1;
    Mytest::round($x);
    is($x, -4, '-4.1 rounds to -4');
}

# Zero stays zero.
{
    my $x = 0;
    Mytest::round($x);
    is($x, 0,  '0 stays 0');
}

# Caller-side variable is mutated even though no value is returned.
{
    my $x = 7.7;
    my $rc = Mytest::round($x);
    is($rc, undef,  'round returns nothing');
    is($x,  8,      '... but mutates its argument');
}

# Arity check: wrong number of args still croaks via Perl_croak_xs_usage.
eval { Mytest::round() };
like($@, qr/Usage:/, 'no-arg round croaks with Usage:');

eval { Mytest::round(1, 2) };
like($@, qr/Usage:/, 'two-arg round croaks with Usage:');

done_testing;
