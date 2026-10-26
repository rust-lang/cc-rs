#include <stddef.h>

/* Two identical tables local to two different functions, like the `aiOff`
   tables in SQLite's FTS3 and FTS5 Unicode folding. From `-O2` on, GCC merges
   such read-only variables into one object, which AddressSanitizer then
   registers once for each table. */

#define TABLE_VALUES 3, 141, 59, 26, 53, 58, 97, 93, 23, 84, 62, 64, 33, 83, 27, 95

/* `index` must be below 16. */
int table_entry(size_t index) {
    static const unsigned short table[] = {TABLE_VALUES};
    return table[index];
}

/* `index` must be below 16. */
int table_entry_from_end(size_t index) {
    static const unsigned short table[] = {TABLE_VALUES};
    return table[sizeof(table) / sizeof(table[0]) - 1 - index];
}
