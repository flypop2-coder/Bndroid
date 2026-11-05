#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/random.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/uio.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr, "ART_PROC_FAILED line=%d errno=%d\n", __LINE__, errno); return 1; } } while (0)

static int read_stat(unsigned long long fields[53]) {
    char line[2048];
    FILE *file = fopen("/proc/self/stat", "re");
    CHECK(file && fgets(line, sizeof(line), file));
    CHECK(fclose(file) == 0);
    CHECK(strtoul(line, NULL, 10) == (unsigned)getpid());
    char *end = strrchr(line, ')');
    CHECK(end && end[1] == ' ' && end[2] == 'R');
    char *next = end + 4;
    for (int field = 4; field <= 52; ++field) fields[field] = strtoull(next, &next, 10);
    CHECK(fields[4] == 1 && fields[20] >= 1 && fields[23] > 0 && fields[24] > 0);
    return 0;
}

int art_proc_checks(void) {
    unsigned long long before[53] = {0}, after[53] = {0};
    CHECK(prctl(PR_SET_NAME, "proc ) test") == 0);
    CHECK(read_stat(before) == 0);
    volatile unsigned long sum = 0;
    struct timespec start, now;
    CHECK(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
    do {
        for (int n = 0; n < 30000; ++n) sum += (unsigned)n;
        CHECK(clock_gettime(CLOCK_MONOTONIC, &now) == 0);
    } while ((now.tv_sec - start.tv_sec) * 1000000000LL + now.tv_nsec - start.tv_nsec < 100000000);
    CHECK(read_stat(after) == 0 && after[14] + after[15] > before[14] + before[15]);
    CHECK(after[22] == before[22]);
    pthread_attr_t attr;
    void *base;
    size_t size;
    CHECK(pthread_getattr_np(pthread_self(), &attr) == 0);
    CHECK(pthread_attr_getstack(&attr, &base, &size) == 0);
    CHECK((uintptr_t)base <= before[28] && before[28] < (uintptr_t)base + size);
    CHECK((uintptr_t)&attr >= (uintptr_t)base && (uintptr_t)&attr < (uintptr_t)base + size);
    CHECK(pthread_attr_destroy(&attr) == 0);
    CHECK(mprotect(base, 4096, PROT_NONE) == 0);
    CHECK(getrandom(base, 1, 0) == -1 && errno == EFAULT);
    FILE *maps = fopen("/proc/self/maps", "re");
    CHECK(maps);
    char line[512];
    int guard = 0;
    while (fgets(line, sizeof(line), maps)) {
        unsigned long lo, hi;
        char rights[5];
        CHECK(sscanf(line, "%lx-%lx %4s", &lo, &hi, rights) == 3);
        CHECK(lo < hi && !(lo <= 0x40080000 && 0x40080000 < hi));
        if (lo == (uintptr_t)base && hi == lo + 4096 && !strcmp(rights, "---p")) guard = 1;
    }
    CHECK(fclose(maps) == 0 && guard);
    CHECK(mprotect(base, 4096, PROT_READ | PROT_WRITE) == 0);
    CHECK(getrandom(base, 1, 0) == 1);
    int fd = open("/proc/self/stat", O_RDONLY | O_CLOEXEC);
    CHECK(fd >= 0);
    int duplicate = dup(fd);
    char bytes[32], again[32];
    CHECK(read(fd, bytes, 17) == 17 && lseek(duplicate, 0, SEEK_CUR) == 17);
    CHECK(pread(duplicate, again, 17, 0) == 17 && !memcmp(bytes, again, 17));
    CHECK(close(fd) == 0 && lseek(duplicate, 0, SEEK_SET) == 0);
    CHECK(read(duplicate, again, 17) == 17 && !memcmp(bytes, again, 17));
    struct stat status;
    CHECK(fstat(duplicate, &status) == 0 && S_ISREG(status.st_mode) && status.st_size == 0);
    CHECK(close(duplicate) == 0);
    CHECK(open("/proc/self/stat", O_WRONLY) == -1 && errno == EROFS);
    CHECK(open("/proc/self/stat", O_DIRECTORY) == -1 && errno == ENOTDIR);
    fd = open("/apex/com.android.art/javalib/core-oj.jar", O_RDONLY);
    CHECK(fd >= 0);
    unsigned char *window = malloc(65557);
    CHECK(window && pread(fd, window, 65557, 0) == 65557 && window[0] == 'P' && window[1] == 'K');
    void *shared = mmap(NULL, 8192, PROT_READ, MAP_SHARED, fd, 4096);
    CHECK(shared != MAP_FAILED);
    unsigned char first = *(unsigned char *)shared;
    for (int advice = MADV_NORMAL; advice <= MADV_SEQUENTIAL; ++advice)
        CHECK(madvise(shared, 8192, advice) == 0 && *(unsigned char *)shared == first);
    CHECK(madvise((char *)shared + 1, 4096, MADV_RANDOM) == -1 && errno == EINVAL);
    CHECK(mprotect(shared, 4096, PROT_READ | PROT_WRITE) == -1 && errno == EACCES);
    CHECK(munmap(shared, 8192) == 0 && close(fd) == 0);
    CHECK(madvise(shared, 8192, MADV_RANDOM) == -1 && errno == ENOMEM);
    free(window);
    unsigned char *old = mmap(NULL, 4096 * 3, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    CHECK(old != MAP_FAILED);
    old[0] = 0x5a;
    old[4096] = 0x7b;
    CHECK(mprotect(old, 4096, PROT_READ) == 0);
    unsigned char *moved = mremap(old, 4096, 8192, MREMAP_MAYMOVE);
    CHECK(moved != MAP_FAILED && moved != old && moved[0] == 0x5a && moved[4096] == 0);
    CHECK(msync(old, 4096, 0) == -1 && errno == ENOMEM);
    CHECK(old[4096] == 0x7b);
    CHECK(getrandom(moved, 1, 0) == -1 && errno == EFAULT);
    CHECK(mremap(moved, 8192, 4096, 0) == moved && moved[0] == 0x5a);
    CHECK(mremap(moved, 4096, 257 * 1024 * 1024, MREMAP_MAYMOVE) == MAP_FAILED && errno == ENOMEM);
    CHECK(moved[0] == 0x5a);
    CHECK(munmap(moved, 4096) == 0 && munmap(old + 4096, 8192) == 0);
    int sock = socket(AF_UNIX, SOCK_DGRAM | SOCK_CLOEXEC, 0);
    CHECK(sock >= 0 && (fcntl(sock, F_GETFD) & FD_CLOEXEC));
    struct sockaddr_un address = {.sun_family = AF_UNIX};
    strcpy(address.sun_path, "/dev/socket/absent");
    CHECK(connect(sock, (void *)&address, sizeof(address)) == -1 && errno == ENOENT);
    strcpy(address.sun_path, "/dev/socket/logdw");
    CHECK(connect(sock, (void *)&address, sizeof(address)) == 0);
    unsigned char header[11] = {0}, priority = 4;
    char tag[] = "bndroid-probe", message[] = "PROC_LOG_TRANSPORT_OK";
    struct iovec vector[] = {{header, sizeof(header)}, {&priority, 1}, {tag, sizeof(tag)}, {message, sizeof(message)}};
    duplicate = dup(sock);
    CHECK(duplicate >= 0 && close(sock) == 0);
    CHECK(writev(duplicate, vector, 4) == (ssize_t)(sizeof(header) + 1 + sizeof(tag) + sizeof(message)));
    CHECK(close(duplicate) == 0);
    (void)sum;
    puts("ART_PROC_OK stack=1 maps=1 counters=1 snapshots=1 large_read=1 shared_ro=1 mremap=1 log=1");
    return 0;
}
