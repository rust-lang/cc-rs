#include <stdint.h>

#ifdef FOO
#if BAR == 1
int32_t foo() {
  return 4;
}
#endif
#endif
