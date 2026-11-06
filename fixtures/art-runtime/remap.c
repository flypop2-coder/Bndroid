#define _GNU_SOURCE
#include <errno.h>
#include <stdint.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/random.h>
#include <ucontext.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr,"ART_REMAP_FAILED line=%d errno=%d\n",__LINE__,errno); return 1; } } while (0)
static void *protected_address;
static volatile sig_atomic_t write_faults;
static void readonly_fault(int number, siginfo_t *info, void *context) {
    if (number != SIGSEGV || info->si_code != SEGV_ACCERR || info->si_addr != protected_address) _exit(99);
    ++write_faults;
    // The deliberately faulting AArch64 STRB below is exactly one instruction.
    ((ucontext_t *)context)->uc_mcontext.pc += 4;
}

int art_remap_checks(void) {
    const size_t page = 4096;
    unsigned char *source = mmap(NULL, 2*page, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    unsigned char *target = mmap((void *)(uintptr_t)0x3000000000ULL, 4*page, PROT_READ|PROT_WRITE,
                                MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE, -1, 0);
    CHECK(source != MAP_FAILED && target == (void *)(uintptr_t)0x3000000000ULL);
    memset(source, 0x53, 2*page);
    memset(target, 0x74, 4*page);
    CHECK(mremap(source, page, page, MREMAP_FIXED, target) == MAP_FAILED && errno == EINVAL);
    CHECK(mremap(source, 2*page, page, MREMAP_FIXED|MREMAP_MAYMOVE, source+page) == MAP_FAILED && errno == EINVAL);
    CHECK(mremap(source, page, page, MREMAP_FIXED|MREMAP_MAYMOVE, target+1) == MAP_FAILED && errno == EINVAL);
    CHECK(mremap(source, page, page, MREMAP_FIXED|MREMAP_MAYMOVE, (void *)(uintptr_t)0x40080000) == MAP_FAILED && errno == ENOMEM);
    CHECK(mremap(source, page, 257*1024*1024, MREMAP_FIXED|MREMAP_MAYMOVE, target) == MAP_FAILED && errno == ENOMEM);
    CHECK(source[0] == 0x53 && source[page] == 0x53 && target[0] == 0x74 && target[3*page] == 0x74);
    CHECK(mprotect(source, page, PROT_READ) == 0);
    unsigned char *moved = mremap(source, page, 2*page, MREMAP_FIXED|MREMAP_MAYMOVE, target+page);
    CHECK(moved == target+page);
    for (size_t n=0; n<page; ++n) CHECK(moved[n] == 0x53 && moved[page+n] == 0);
    CHECK(target[0] == 0x74 && target[3*page] == 0x74 && source[page] == 0x53);
    CHECK(getrandom(moved, 1, 0) == -1 && errno == EFAULT);
    CHECK(mprotect(source, page, PROT_READ) == -1 && errno == ENOMEM);
    struct sigaction action = {.sa_sigaction = readonly_fault, .sa_flags = SA_SIGINFO}, previous;
    sigemptyset(&action.sa_mask);
    protected_address = moved;
    CHECK(sigaction(SIGSEGV, &action, &previous) == 0);
    __asm__ volatile("strb %w1, [%0]" :: "r"(moved), "r"(255) : "memory");
    CHECK(write_faults == 1 && moved[0] == 0x53);
    CHECK(sigaction(SIGSEGV, &previous, NULL) == 0);
    CHECK(mremap(moved, 2*page, page, MREMAP_FIXED|MREMAP_MAYMOVE, source) == source);
    CHECK(source[0] == 0x53 && source[page] == 0x53);
    CHECK(mremap(source, page, page, MREMAP_FIXED|MREMAP_MAYMOVE, moved) == moved);
    CHECK(moved[0] == 0x53 && target[0] == 0x74 && target[3*page] == 0x74);
    CHECK(getrandom(moved, 1, 0) == -1 && errno == EFAULT);
    CHECK(mprotect(source, page, PROT_READ) == -1 && errno == ENOMEM);
    CHECK(munmap(source, 2*page) == 0 && munmap(target, 4*page) == 0);
    puts("ART_REMAP_OK fixed=1 replaced=1 readonly=1 growth_zero=1 source_unmapped=1 atomic_failure=1");
    return 0;
}
