use utf8;
use Test2::V0;

# xs_boot! multi-package form: one shared library, one boot function
# (boot_Mytest2, which XSLoader::load('Mytest2') calls), two Perl
# namespaces. See ../src/lib.rs and
# libperl-macros/src/xs_boot.rs "Multiple packages".
use Mytest2;

ok(defined &Mytest2::foo,          'first package is still registered');
ok(defined &Mytest2::Extra::twice, 'second package registered by the same boot');
ok(defined &Mytest2::Extra::tag,   'second package: all listed subs registered');

# Namespaces really are separate — subs do not leak either way.
ok(!defined &Mytest2::twice,      'second-package sub is not in the first package');
ok(!defined &Mytest2::Extra::foo, 'first-package sub is not in the second package');

is(Mytest2::Extra::twice(21),  42, 'Extra::twice(21) = 42');
is(Mytest2::Extra::twice(-3),  -6, 'Extra::twice(-3) = -6');
is(Mytest2::Extra::tag("x"), 'extra:x', 'Extra::tag round-trips a string');
is(Mytest2::Extra::tag(""),  'extra:',  'Extra::tag on empty string');

# #[xs_sub]'s arity check is unaffected by which package the sub landed in.
like(dies { Mytest2::Extra::twice() },    qr/Usage:/, 'wrong-arity twice croaks');
like(dies { Mytest2::Extra::tag(1, 2) },  qr/Usage:/, 'wrong-arity tag croaks');

done_testing;
