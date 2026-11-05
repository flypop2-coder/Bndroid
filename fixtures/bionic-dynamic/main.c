#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { dprintf(2, "DYNAMIC_CHECK_FAILED line=%d errno=%d\n", __LINE__, errno); exit(1); } } while (0)
static volatile _Thread_local int local = 17;
static volatile int constructor_seen;
__attribute__((constructor)) static void before_main(void) { constructor_seen = 1; }
__attribute__((destructor)) static void after_main(void) { puts("DYNAMIC_DESTRUCTOR_OK"); }
static void at_exit(void) { puts("DYNAMIC_ATEXIT_OK"); }
#ifdef BNDROID_TEST_THREADS
extern void test_threads(void);
#endif

int main(int argc, char **argv) {
    CHECK(argc == 2 && strcmp(argv[1], "--verify") == 0);
    CHECK(constructor_seen == 1 && local == 17);
    local = 39;
    CHECK(atexit(at_exit) == 0);
    puts("DYNAMIC_MAIN_ENTERED");
    void *plugin = dlopen("libbndroidplugin.so", RTLD_NOW | RTLD_LOCAL);
    if (!plugin) dprintf(2, "DYNAMIC_DLOPEN_ERROR %s\n", dlerror());
    CHECK(plugin != NULL);
    int (*compute)(int) = (int (*)(int))dlsym(plugin, "bndroid_plugin_compute");
    CHECK(compute && compute(6) == 42 && compute(7) == 44);
    CHECK(dlsym(plugin, "no_such_export") == NULL && dlerror() != NULL);
    CHECK(dlopen("no-such-library.so", RTLD_NOW) == NULL && dlerror() != NULL);
    CHECK(local == 39);
    for (int tick = 0; tick < 3; ++tick) __asm__ volatile("wfi" ::: "memory");
    CHECK(local == 39 && compute(11) == 49);
    Dl_info location;
    CHECK(dladdr((void *)malloc, &location) != 0 && strstr(location.dli_fname, "libc.so") != NULL);
    volatile double value = 0.0;
    CHECK(cos(value) == 1.0);
    CHECK(dlclose(plugin) == 0);
#ifdef BNDROID_TEST_THREADS
    test_threads();
#endif

    int directory = open("/system/etc", O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    CHECK(directory >= 3);
    int fd = openat(directory, "runtime-data", O_RDONLY | O_CLOEXEC);
    CHECK(fd >= 3);
    struct stat info;
    CHECK(fstat(fd, &info) == 0 && S_ISREG(info.st_mode) && info.st_size == 8192);
    CHECK(fcntl(fd, F_GETFD) == FD_CLOEXEC);
    int duplicate = dup(fd);
    CHECK(duplicate >= 3 && fcntl(duplicate, F_GETFD) == 0);
    char bytes[16];
    CHECK(read(fd, bytes, 4) == 4 && memcmp(bytes, "BNDR", 4) == 0);
    CHECK(lseek(duplicate, 0, SEEK_CUR) == 4);
    CHECK(pread(duplicate, bytes, 4, 0) == 4 && memcmp(bytes, "BNDR", 4) == 0);
    CHECK(lseek(fd, 0, SEEK_CUR) == 4);
    volatile uintptr_t forbidden = 0x40080000;
    CHECK(read(fd, (void *)forbidden, 1) == -1 && errno == EFAULT);
    CHECK(lseek(fd, 0, SEEK_CUR) == 4);
    CHECK(fstat(fd, (struct stat *)forbidden) == -1 && errno == EFAULT);
    CHECK(lseek(fd, 8192, SEEK_SET) == 8192 && read(fd, bytes, 1) == 0);
    CHECK(close(duplicate) == 0);
    unsigned char *mapped = mmap(NULL, 8192, PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0);
    CHECK(mapped != MAP_FAILED && memcmp(mapped, "BNDR", 4) == 0 && mapped[8191] == 0x5a);
    mapped[0] = 0xff;
    CHECK(pread(fd, bytes, 4, 0) == 4 && bytes[0] == 'B');
    CHECK(mprotect(mapped, 8192, PROT_READ) == 0 && munmap(mapped, 8192) == 0);
    CHECK(mmap(NULL, 4096, PROT_WRITE | PROT_EXEC, MAP_PRIVATE, fd, 0) == MAP_FAILED);
    CHECK(mmap(NULL, 4096, PROT_READ, MAP_PRIVATE, fd, 1) == MAP_FAILED);
    unsigned char *reserved = mmap(NULL, 12288, PROT_NONE, MAP_PRIVATE, fd, 0);
    CHECK(reserved != MAP_FAILED);
    CHECK(mprotect(reserved, 12288, PROT_READ) == 0 && reserved[0] == 'B' && reserved[8191] == 0x5a);
#ifdef BNDROID_TEST_EOF_FAULT
    printf("DYNAMIC_EOF_FAULT_ARMED address=%p\n", reserved + 8192);
    fflush(stdout);
    volatile unsigned char unavailable = reserved[8192];
    (void)unavailable;
    puts("DYNAMIC_EOF_FAULT_MISSED");
    return 2;
#endif
    CHECK(munmap(reserved, 12288) == 0);
    CHECK(close(fd) == 0 && close(directory) == 0);
    CHECK(open("/system/etc/runtime-data", O_WRONLY) == -1 && errno == EROFS);
    CHECK(open("/system/etc/runtime-data/child", O_RDONLY) == -1 && errno == ENOTDIR);
    CHECK(open("/missing/../system/etc/runtime-data", O_RDONLY) == -1 && errno == ENOENT);
    CHECK(stat("/system/etc/runtime-link", &info) == 0 && S_ISREG(info.st_mode));
    CHECK(lstat("/system/etc/runtime-link", &info) == 0 && S_ISLNK(info.st_mode));
    CHECK(readlink("/system/etc/runtime-link", bytes, sizeof(bytes)) == 12 && memcmp(bytes, "runtime-data", 12) == 0);
    CHECK(open("/system/etc/runtime-link", O_RDONLY | O_NOFOLLOW) == -1 && errno == ELOOP);
    // Deliberately leave one open description: process reaping must release it.
    CHECK(open("/system/etc/runtime-data", O_RDONLY) >= 3);
    int descriptors[64], used = 0;
    while (used < 64) {
        int next = open("/system/etc/runtime-data", O_RDONLY);
        if (next < 0) break;
        descriptors[used++] = next;
    }
    CHECK(used == 60 && errno == EMFILE);
    for (int index = 0; index < used; ++index) CHECK(close(descriptors[index]) == 0);
    char *heap = calloc(1, 65536);
    CHECK(heap != NULL && heap[65535] == 0);
    heap[65535] = 9;
    free(heap);
    puts("DYNAMIC_PROBE_OK linker=android shared_libc=1 dlopen=1 tls=1 files=1 private_mmap=1");
    return 0;
}
