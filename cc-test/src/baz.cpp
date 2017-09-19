#include <stdint.h>

extern "C" int32_t
baz() {
  int *a = new int(8);
  int b = *a;
  delete a;
  return b;
}
