#include <errno.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/mman.h>
#include <sys/random.h>
#include <unistd.h>

#define CHECK(value) do { if (!(value)) { fprintf(stderr, "ART_MEMORY_FAILED line=%d errno=%d\n", __LINE__, errno); return 1; } } while (0)

static void *worker(void *argument) {
    uint32_t *shared = argument;
    *shared = 0x1234abcd;
    return argument;
}
int art_proc_checks(void);
int art_paging_checks(void);
int art_remap_checks(void);

int main(void) {
    CHECK(art_proc_checks() == 0);
    CHECK(art_paging_checks() == 0);
    CHECK(art_remap_checks() == 0);
    const size_t page = 4096, size = 64 * 1024 * 1024;
    unsigned char *low = mmap((void *)0x02000000, size, PROT_READ | PROT_WRITE,
                             MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    CHECK(low != MAP_FAILED && (uintptr_t)low >= 0x80000000 && (uintptr_t)low + size <= 0x100000000ULL);
    for (size_t at = 0; at < size; at += page) {
        CHECK(low[at] == 0);
        low[at] = (unsigned char)(at / page);
    }
    for (size_t at = 0; at < size; at += page) CHECK(low[at] == (unsigned char)(at / page));
    CHECK(msync(low, size, MS_SYNC) == 0);
    CHECK(mprotect(low, page, PROT_READ) == 0);
    CHECK(getrandom(low, 16, 0) == -1 && errno == EFAULT);
    CHECK(mprotect(low, page, PROT_READ | PROT_WRITE) == 0);
    CHECK(munmap(low + page, page) == 0);
    CHECK(msync(low, page * 3, 0) == -1 && errno == ENOMEM);
    CHECK(mmap(low + page, page, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0) == low + page);
    CHECK(low[page] == 0 && low[page * 2] == 2);
    CHECK(msync(low + 1, page, 0) == -1 && errno == EINVAL);
    CHECK(msync(low, page, MS_ASYNC | MS_SYNC) == -1 && errno == EINVAL);
    CHECK(mmap((void *)0x40080000, page, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0) == MAP_FAILED && errno == ENOMEM);
    CHECK(msync((void *)0x40080000, page, 0) == -1 && errno == ENOMEM);
    CHECK(mmap((void *)0xfffff000, page * 2, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0) == MAP_FAILED && errno == ENOMEM);
    CHECK(mmap(NULL, 257 * 1024 * 1024, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) == MAP_FAILED && errno == ENOMEM);
    uint32_t *shared = mmap((void *)0x04000000, page, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    CHECK(shared != MAP_FAILED && (uintptr_t)shared < 0x100000000ULL && *shared == 0);
    CHECK(madvise(shared, page, MADV_WIPEONFORK) == -1 && errno == EINVAL);
    pthread_t thread;
    CHECK(pthread_create(&thread, NULL, worker, shared) == 0);
    void *result = NULL;
    CHECK(pthread_join(thread, &result) == 0 && result == shared && *shared == 0x1234abcd);
    CHECK(msync(shared, page, MS_SYNC) == 0);
    CHECK(munmap(shared, page) == 0);
    // Leave the 64 MiB owner for process reaping, exercising all low L1 leaves.
    puts("ART_MEMORY_OK low_64m=1 protections=1 holes=1 shared_threads=1 kernel_isolation=1");
    return 0;
}
