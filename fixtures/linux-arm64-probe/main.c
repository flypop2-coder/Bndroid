/* Compiled by the installed Android NDK, using the Linux arm64 UAPI.
 * This first entry probe supplies its own crt; it does not replace Bionic. */
#include <asm/unistd.h>
#include <linux/auxvec.h>
#include <linux/errno.h>
#include <stdint.h>
#include <sys/utsname.h>

static volatile uint64_t initialized = 0x123456789abcdef0ULL;
static volatile uint64_t zeroed[19];

static long call(long number, long a, long b, long c) {
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x8 __asm__("x8") = number;
    __asm__ volatile("svc #0" : "+r"(x0) : "r"(x1), "r"(x2), "r"(x8) : "memory", "cc");
    return x0;
}

static int equal(const char *a, const char *b) {
    while (*a && *a == *b) { ++a; ++b; }
    return *a == *b;
}

static int write_all(const char *bytes, long length) {
    while (length) {
        long count = call(__NR_write, 1, (long)bytes, length);
        if (count <= 0 || count > length) return 0;
        bytes += count;
        length -= count;
    }
    return 1;
}

#define CHECK(condition, code) do { if (!(condition)) return (code); } while (0)

int linux_probe_main(const uint64_t *stack) {
    uint64_t initial_tls, read_only_tls;
    __asm__ volatile("mrs %0, tpidr_el0\n mrs %1, tpidrro_el0" : "=r"(initial_tls), "=r"(read_only_tls));
    CHECK(initial_tls == 0 && read_only_tls == 0, 47);
    uint64_t tls_cookie = (uint64_t)(uintptr_t)zeroed;
    __asm__ volatile("msr tpidr_el0, %0" :: "r"(tls_cookie) : "memory");
    for (int tick = 0; tick < 3; ++tick) __asm__ volatile("wfi" ::: "memory");
    __asm__ volatile("mrs %0, tpidr_el0" : "=r"(initial_tls));
    CHECK(initial_tls == tls_cookie, 48);
    CHECK(((uintptr_t)stack & 15) == 0 && stack[0] == 2, 10);
    CHECK(equal((const char *)stack[1], "/system/bin/linux-probe"), 11);
    CHECK(equal((const char *)stack[2], "--verify") && stack[3] == 0, 12);
    CHECK(equal((const char *)stack[4], "LANG=C") && stack[5] == 0, 13);
    uint64_t page_size = 0, entry = 0, phdr = 0, phnum = 0, phent = 0, execfn = 0;
    for (const uint64_t *aux = stack + 6; aux[0]; aux += 2) {
        switch (aux[0]) {
            case AT_PAGESZ: page_size = aux[1]; break;
            case AT_ENTRY: entry = aux[1]; break;
            case AT_PHDR: phdr = aux[1]; break;
            case AT_PHENT: phent = aux[1]; break;
            case AT_PHNUM: phnum = aux[1]; break;
            case AT_EXECFN: execfn = aux[1]; break;
        }
    }
    CHECK(page_size == 4096 && entry != 0 && phdr != 0 && phnum >= 3 && phent == 56, 14);
    CHECK(execfn == stack[1] && *(const uint32_t *)phdr == 6, 15); /* PT_PHDR */
    CHECK(initialized == 0x123456789abcdef0ULL, 16);
    for (int index = 0; index != 19; ++index) CHECK(zeroed[index] == 0, 17);
    zeroed[18] = 42;
    CHECK(zeroed[18] == 42, 18);

    long pid = call(__NR_getpid, 0, 0, 0);
    CHECK(pid > 1 && call(__NR_gettid, 0, 0, 0) == pid, 20);
    CHECK(call(__NR_getppid, 0, 0, 0) == 1, 21);
    CHECK(call(__NR_getuid, 0, 0, 0) == 10000 && call(__NR_geteuid, 0, 0, 0) == 10000, 22);
    CHECK(call(__NR_getgid, 0, 0, 0) == 10000 && call(__NR_getegid, 0, 0, 0) == 10000, 23);
    /* These overlap native Bndroid operations and must never reach them. */
    CHECK(call(0, 0, 0, 0) == -ENOSYS, 24);
    CHECK(call(1, 0, 0, 0) == -ENOSYS && call(2, 0, 0, 0) == -ENOSYS, 25);
    CHECK(call(0x7fffffff, 0, 0, 0) == -ENOSYS, 26);
    register long rejected __asm__("x0") = 0;
    __asm__ volatile("svc #0x42" : "+r"(rejected) :: "memory", "cc");
    CHECK(rejected == -ENOSYS, 27);

    CHECK(call(__NR_write, 7, 0, 0) == -EBADF, 30);
    CHECK(call(__NR_write, 1, 0, 1) == -EFAULT, 31);
    CHECK(call(__NR_write, 1, -1, 2) == -EFAULT, 32);
    CHECK(call(__NR_write, 1, 0, 0) == 0, 33);
    CHECK(call(__NR_write, 1, 0x40000000, 1) == -EFAULT, 44);
    int64_t first[2], second[2];
    CHECK(call(__NR_clock_gettime, 1, (long)first, 0) == 0, 34);
    CHECK(call(__NR_clock_gettime, 1, (long)second, 0) == 0, 35);
    CHECK(first[0] >= 0 && first[1] >= 0 && first[1] < 1000000000, 36);
    CHECK(second[0] > first[0] || (second[0] == first[0] && second[1] >= first[1]), 37);
    CHECK(call(__NR_clock_gettime, -1, (long)first, 0) == -EINVAL, 38);
    CHECK(call(__NR_clock_gettime, 1, (long)"read-only", 0) == -EFAULT, 39);
    struct utsname name;
    CHECK(call(__NR_uname, (long)&name, 0, 0) == 0, 40);
    CHECK(equal(name.machine, "aarch64") && equal(name.sysname, "Bndroid"), 41);
    CHECK(call(__NR_uname, 0, 0, 0) == -EFAULT, 42);
    char bounded_write[301];
    for (int index = 0; index < 300; ++index) bounded_write[index] = '.';
    bounded_write[300] = '\n';
    CHECK(call(__NR_write, 2, (long)bounded_write, sizeof(bounded_write)) == 256, 45);
    CHECK(call(__NR_write, 2, (long)(bounded_write + 256), 45) == 45, 46);
    static const char message[] = "LINUX_ARM64_PROBE_OK entry=elf stack=linux syscalls=linux memory=isolated\n";
    CHECK(write_all(message, sizeof(message) - 1), 43);
    __asm__ volatile("mrs %0, tpidr_el0" : "=r"(initial_tls));
    CHECK(initial_tls == tls_cookie, 49);
    return 0;
}
