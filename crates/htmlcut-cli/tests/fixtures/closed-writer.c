// SPDX-License-Identifier: MPL-2.0
#include <unistd.h>
#include <sys/stat.h>
#include <stddef.h>

/* Test-only Darwin interposition: close the actual stdout-backed descriptor
   immediately before its first nonempty write. The kernel produces EBADF;
   this fixture never fabricates a syscall result or changes application code. */
static ssize_t close_then_write(int fd, const void *bytes, size_t count) {
    static int closed;
    struct stat target, standard;
    if (!closed && count && fd != STDERR_FILENO &&
        fstat(fd, &target) == 0 && fstat(STDOUT_FILENO, &standard) == 0 &&
        target.st_dev == standard.st_dev && target.st_ino == standard.st_ino) {
        closed = 1;
        close(fd);
    }
    return write(fd, bytes, count);
}
__attribute__((used, section("__DATA,__interpose")))
static const struct { const void *replacement; const void *original; } interposition = {
    (const void *)close_then_write, (const void *)write
};
