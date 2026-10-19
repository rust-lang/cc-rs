#include <stdlib.h>

/* Reads `buf[index]` of an 8 byte heap buffer. An index of 8 reads one byte
   past its end, which AddressSanitizer reports only if this file is
   instrumented. */
int read_heap(size_t index) {
    char *buf = malloc(8);
    if (buf == NULL) {
        return -1;
    }
    for (size_t i = 0; i < 8; i++) {
        buf[i] = (char)i;
    }
    int value = ((volatile char *)buf)[index];
    free(buf);
    return value;
}
