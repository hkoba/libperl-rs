use utf8;
use Test2::V0;

use Mytest2;

# `&Perl` context arg + `Sv::new_*` constructors (Phase 3.10c).
# These XS subs allocate a fresh SV inside the Rust body and return it.
# Mortal-forced policy means the caller doesn't need to fiddle with
# refcounts; the SV is freed at end of expression unless someone takes
# a ref.

is(Mytest2::wrap_iv(42),       42,       'wrap_iv: positive IV round-trip');
is(Mytest2::wrap_iv(-7),       -7,       'wrap_iv: negative IV round-trip');
is(Mytest2::wrap_iv(0),        0,        'wrap_iv: zero');

is(Mytest2::wrap_uv(0),        0,        'wrap_uv: zero');
is(Mytest2::wrap_uv(2**31),    2**31,    'wrap_uv: large');

cmp_ok(Mytest2::wrap_nv(3.5),  '==', 3.5,   'wrap_nv: positive NV');
cmp_ok(Mytest2::wrap_nv(-0.25),'==', -0.25, 'wrap_nv: negative NV');

is(Mytest2::wrap_pv("hello"),  "hello",  'wrap_pv: ASCII');
is(Mytest2::wrap_pv("日本語"), "日本語",  'wrap_pv: UTF-8 round-trip');
ok(utf8::is_utf8(Mytest2::wrap_pv("日本語")), 'wrap_pv: UTF-8 flag set');

# Arity check still works through the new `&Perl` plumbing — the
# context arg must NOT consume a Perl-side stack slot.
like(dies { Mytest2::wrap_iv() },     qr/Usage:/, 'wrap_iv with no args croaks');
like(dies { Mytest2::wrap_iv(1, 2) }, qr/Usage:/, 'wrap_iv with extra arg croaks');

# 1000-call leak smoke check.
{
    Mytest2::wrap_pv("leak-check") for 1..1000;
    pass('1000 wrap_pv calls ran without abort');
}

done_testing;
