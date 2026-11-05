/* Uses the installed NDK's unmodified crt, libc, allocator, and TLS runtime. */
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/random.h>
#include <sys/resource.h>
#include <time.h>
#include <unistd.h>

static _Thread_local volatile unsigned tls_initialized = 73;
static _Thread_local volatile unsigned tls_zero;
static volatile unsigned constructed;
static volatile unsigned cleanup_done;

#define CHECK(expression) do { if (!(expression)) { \
    fprintf(stderr, "BIONIC_CHECK_FAILED line=%d errno=%d\n", __LINE__, errno); \
    return __LINE__; \
} } while (0)

static void output(const char *message) {
    size_t remaining = strlen(message);
    while (remaining) {
        ssize_t done = write(STDOUT_FILENO, message, remaining);
        if (done <= 0 || (size_t)done > remaining) _exit(201);
        message += done;
        remaining -= (size_t)done;
    }
}

__attribute__((constructor)) static void before_main(void) {
    if (tls_initialized != 73 || tls_zero != 0) _exit(202);
    constructed = 47;
}

static void on_exit(void) {
    if (tls_initialized != 91 || tls_zero != 19) _exit(203);
    cleanup_done = 1;
    output("BIONIC_ATEXIT_OK\n");
}

__attribute__((destructor)) static void after_main(void) {
    if (cleanup_done != 1) _exit(204);
    output("BIONIC_DESTRUCTOR_OK\n");
}

int main(int argc, char **argv) {
    CHECK(argc == 2 && strcmp(argv[0], "/system/bin/bionic-probe") == 0);
    CHECK(strcmp(argv[1], "--verify") == 0 && constructed == 47);
    CHECK(getauxval(AT_PAGESZ) == 4096 && getauxval(AT_RANDOM) != 0);
    CHECK(tls_initialized == 73 && tls_zero == 0);
    tls_initialized = 91;
    tls_zero = 19;
    errno = EDOM;
    for (int iteration = 0; iteration != 3; ++iteration) {
        __asm__ volatile("wfi" ::: "memory");
        CHECK(getpid() > 1 && errno == EDOM);
        CHECK(tls_initialized == 91 && tls_zero == 19);
    }
    CHECK(close(999) == -1 && errno == EBADF);
    struct timespec time;
    CHECK(clock_gettime(CLOCK_MONOTONIC, &time) == 0 && time.tv_nsec < 1000000000);
    CHECK(fcntl(STDOUT_FILENO, F_GETFL) == O_WRONLY);
    CHECK(fcntl(STDOUT_FILENO, F_SETFD, FD_CLOEXEC) == 0);
    CHECK(fcntl(STDOUT_FILENO, F_GETFD) == FD_CLOEXEC);
    CHECK(fcntl(STDOUT_FILENO, F_SETFD, 0) == 0);
    struct rlimit limit_info;
    CHECK(getrlimit(RLIMIT_NOFILE, &limit_info) == 0 && limit_info.rlim_cur == 3 && limit_info.rlim_max == 3);
    CHECK(getrlimit(RLIMIT_STACK, &limit_info) == 0 && limit_info.rlim_cur >= 4 * 4096);

    char *memory = calloc(1024, 1);
    CHECK(memory != NULL);
    volatile unsigned char *zero_check = (volatile unsigned char *)memory;
    for (int index = 0; index < 1024; ++index) CHECK(zero_check[index] == 0);
    strcpy(memory, "real Bionic allocation");
    memory = realloc(memory, 8192);
    CHECK(memory != NULL && strcmp(memory, "real Bionic allocation") == 0);
    free(memory);
    size_t large_size = 2 * 1024 * 1024;
    volatile unsigned char *large = malloc(large_size);
    CHECK(large != NULL);
    for (size_t index = 0; index < large_size; index += 4096) large[index] = (unsigned char)(index / 4096);
    for (size_t index = 0; index < large_size; index += 4096) CHECK(large[index] == (unsigned char)(index / 4096));
    free((void *)large);

    unsigned char first_random[32], next_random[32];
    CHECK(getrandom(first_random, sizeof(first_random), 0) == sizeof(first_random));
    CHECK(getrandom(next_random, sizeof(next_random), 0) == sizeof(next_random));
    CHECK(memcmp(first_random, next_random, sizeof(first_random)) != 0);
    CHECK(getrandom(first_random, 1, 0x80) == -1 && errno == EINVAL);

    unsigned char *region = mmap(NULL, 3 * 4096, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    CHECK(region != MAP_FAILED);
    CHECK(write(STDOUT_FILENO, region, 1) == -1 && errno == EFAULT);
    CHECK(mprotect(region + 4096, 4096, PROT_READ | PROT_WRITE) == 0);
    region[4096] = 19;
    region[8191] = 81;
    CHECK(mprotect(region + 4096, 4096, PROT_READ) == 0);
    CHECK(getrandom(region + 4096, 1, 0) == -1 && errno == EFAULT);
    CHECK(mprotect(region + 4096, 4096, PROT_READ | PROT_WRITE) == 0);
    CHECK(region[4096] == 19 && region[8191] == 81);
    CHECK(mmap(region + 4096, 4096, PROT_READ, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0) == MAP_FAILED && errno == EEXIST);
    CHECK(munmap(region + 4096, 4096) == 0);
    CHECK(mprotect(region, 3 * 4096, PROT_READ | PROT_WRITE) == -1 && errno == ENOMEM);
    CHECK(write(STDOUT_FILENO, region, 1) == -1 && errno == EFAULT);
    CHECK(mmap(region + 4096, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0) == region + 4096);
    CHECK(region[4096] == 0 && region[8191] == 0);
    CHECK(mmap(NULL, 4096, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) == MAP_FAILED && errno == EACCES);
    CHECK(mmap((void *)0x40000000, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0) == MAP_FAILED);
    CHECK(munmap(region, 3 * 4096) == 0);
    CHECK(getrandom(region, 1, 0) == -1 && errno == EFAULT);

    void *limit = (void *)0x7000000000ULL;
    CHECK(mmap(limit, 1UL << 30, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0) == MAP_FAILED && errno == ENOMEM);
    CHECK(mmap(limit, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0) == limit);
    CHECK(munmap(limit, 4096) == 0);

    CHECK(prctl(PR_GET_TAGGED_ADDR_CTRL, 0UL, 0UL, 0UL, 0UL) == PR_TAGGED_ADDR_ENABLE);
    uintptr_t tagged = ((uintptr_t)first_random & 0x00ffffffffffffffULL) | 0xa500000000000000ULL;
    CHECK(getrandom((void *)tagged, 16, 0) == 16);
    CHECK(prctl(PR_SET_TAGGED_ADDR_CTRL, 0UL, 0UL, 0UL, 0UL) == 0);
    CHECK(getrandom((void *)tagged, 1, 0) == -1 && errno == EFAULT);
    CHECK(prctl(PR_SET_TAGGED_ADDR_CTRL, 1UL, 0UL, 0UL, 0UL) == 0);
    CHECK(getrandom((void *)0xab00000040000000ULL, 1, 0) == -1 && errno == EFAULT);
    CHECK(prctl(PR_SET_TAGGED_ADDR_CTRL, 2UL, 0UL, 0UL, 0UL) == -1 && errno == EINVAL);
    CHECK(atexit(on_exit) == 0);
    CHECK(read(STDIN_FILENO, first_random, 1) == 0);
    CHECK(close(STDIN_FILENO) == 0);
    CHECK(read(STDIN_FILENO, first_random, 1) == -1 && errno == EBADF);
    printf("BIONIC_PROBE_OK constructors=1 tls=1 errno=1 allocator=1 vm=1 random=1 tagged_pointers=1 stdio=1\n");
    return 0;
}
