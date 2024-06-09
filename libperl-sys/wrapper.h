#include <INTERN.h>               /* from the Perl distribution     */
#define PERL_IN_GLOBALS_C
#include <perl.h>                 /* from the Perl distribution     */

#if    defined(G_ARRAY) && !defined(G_LIST)
#  define G_LIST   G_ARRAY
#elif !defined(G_ARRAY)
#  define G_ARRAY  G_LIST
#endif
