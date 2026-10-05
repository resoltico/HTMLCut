// SPDX-License-Identifier: MPL-2.0
#include <fcntl.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>

/* Replace only the synthetic fixture immediately before the real kernel open. */
static void replace_input(const char *path, int flags) {
    static int replaced;
    const char *target = getenv("HTMLCUT_RACE_PATH");
    const char *marker = getenv("HTMLCUT_RACE_MARKER");
    if (!replaced && target && marker && strcmp(path, target) == 0) {
        replaced = 1;
        if (unlink(path) != 0 || mkfifo(path, 0600) != 0) _exit(91);
        int record = open(marker, O_WRONLY | O_CREAT | O_TRUNC, 0600);
        if (record < 0) _exit(92);
        const char *value = flags & O_NONBLOCK ? "replaced;nonblocking" : "replaced;blocking";
        size_t size = strlen(value);
        if (write(record, value, size) != (ssize_t)size) _exit(93);
        close(record);
    }
}
static int race_open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) { va_list args; va_start(args, flags); mode = (mode_t)va_arg(args, int); va_end(args); }
    replace_input(path, flags);
    return open(path, flags, mode);
}
static int race_openat(int directory, const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) { va_list args; va_start(args, flags); mode = (mode_t)va_arg(args, int); va_end(args); }
    replace_input(path, flags);
    return openat(directory, path, flags, mode);
}
#define INTERPOSE(replacement, original, name) \
    __attribute__((used, section("__DATA,__interpose"))) \
    static const struct { const void *replacement; const void *original; } name = { \
        (const void *)replacement, (const void *)original }
INTERPOSE(race_open, open, open_pair);
INTERPOSE(race_openat, openat, openat_pair);
