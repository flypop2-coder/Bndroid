#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/random.h>
#include <sys/stat.h>
#include <sys/vfs.h>
#include <sys/syscall.h>
#include <sys/utsname.h>
#include <ucontext.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr, "ART_MEMFD_FAILED line=%d errno=%d\n", __LINE__, errno); return 1; } } while (0)
static volatile sig_atomic_t faults;
static void *fault_address;
static int fault_signal, fault_code;
static int execute_fault;
static void catch_fault(int signal, siginfo_t *info, void *context) {
    if (signal != fault_signal || info->si_code != fault_code || info->si_addr != fault_address) _exit(90);
    ++faults;
    if (execute_fault) ((ucontext_t *)context)->uc_mcontext.pc = ((ucontext_t *)context)->uc_mcontext.regs[30];
    else ((ucontext_t *)context)->uc_mcontext.pc += 4;
}
static int new_file(const char *name, unsigned flags) { return (int)syscall(__NR_memfd_create, name, flags); }

int main(void) {
    const size_t page = 4096;
    struct utsname name;
    CHECK(uname(&name) == 0 && strcmp(name.release, "3.17.0-bndroid-abi") == 0);
    CHECK(new_file("invalid", 4) == -1 && errno == EINVAL);
    CHECK(new_file((void *)(uintptr_t)0x40080000, 0) == -1 && errno == EFAULT);
    char too_long[251]; memset(too_long, 'a', 250); too_long[250] = 0;
    CHECK(new_file(too_long, 0) == -1 && errno == EINVAL);
    int fd = new_file("bndroid-code", MFD_CLOEXEC | MFD_ALLOW_SEALING);
    CHECK(fd >= 0 && fcntl(fd, F_GETFD) == FD_CLOEXEC && fcntl(fd, F_GET_SEALS) == 0);
    int duplicate = dup(fd);
    CHECK(duplicate >= 0 && fcntl(duplicate, F_GETFD) == 0);
    CHECK(ftruncate(fd, 2*page) == 0);
    struct stat status;
    CHECK(fstat(duplicate, &status) == 0 && S_ISREG(status.st_mode) && status.st_size == (off_t)(2*page));
    CHECK(status.st_uid == getuid() && status.st_gid == getgid() && status.st_nlink == 0);
    struct statfs filesystem;
    CHECK(fstatfs(fd, &filesystem) == 0 && filesystem.f_type == 0x01021994 && filesystem.f_bsize == (long)page);
    char path[64], link[64];
    snprintf(path, sizeof(path), "/proc/self/fd/%d", fd);
    ssize_t link_size = readlink(path, link, sizeof(link));
    const char expected_link[] = "/memfd:bndroid-code (deleted)";
    CHECK(link_size == (ssize_t)strlen(expected_link) && memcmp(link, expected_link, (size_t)link_size) == 0);
    unsigned char *rw = mmap(NULL, 3*page, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0);
    unsigned char *rx = mmap(NULL, 3*page, PROT_READ|PROT_EXEC, MAP_SHARED, duplicate, 0);
    CHECK(rw != MAP_FAILED && rx != MAP_FAILED && rw != rx);
    CHECK(madvise(rw, 3*page, MADV_DONTFORK) == 0 && madvise(rw, 3*page, MADV_DOFORK) == 0);
    rw[page] = 0x58; CHECK(rx[page] == 0x58);
    CHECK(pwrite(fd, "hello", 5, 100) == 5 && memcmp(rx+100, "hello", 5) == 0);
    CHECK(lseek(fd, 100, SEEK_SET) == 100 && read(duplicate, link, 5) == 5 && memcmp(link, "hello", 5) == 0);
    CHECK(lseek(fd, 0, SEEK_CUR) == 105);
    CHECK(pread(fd, link, 5, 100) == 5 && lseek(fd, 0, SEEK_CUR) == 105);
    CHECK(getrandom(rx, 1, 0) == -1 && errno == EFAULT);
    CHECK(mprotect(rx, page, PROT_READ|PROT_WRITE|PROT_EXEC) == -1 && errno == EACCES);
    CHECK(mmap(NULL, page, PROT_READ|PROT_WRITE|PROT_EXEC, MAP_SHARED, fd, 0) == MAP_FAILED && errno == EACCES);
    CHECK(mmap(NULL, page, PROT_READ, MAP_PRIVATE, fd, 0) == MAP_FAILED && errno == ENOSYS);
    CHECK(ftruncate(fd, 129*1024*1024) == -1 && errno == EFBIG && rx[page] == 0x58);
    // Real generated AArch64 instructions: mov w0,#42; ret. User cache
    // maintenance operates on both aliases, as ART does for dual-view code.
    const uint32_t code[] = {0x52800540, 0xd65f03c0};
    memcpy(rw, code, sizeof(code));
    __builtin___clear_cache((char *)rw, (char *)rw+sizeof(code));
    __builtin___clear_cache((char *)rx, (char *)rx+sizeof(code));
    CHECK(((int (*)(void))rx)() == 42);
    uint32_t replacement = 0x52800560; // mov w0,#43
    memcpy(rw, &replacement, sizeof(replacement));
    __builtin___clear_cache((char *)rw, (char *)rw+sizeof(code));
    __builtin___clear_cache((char *)rx, (char *)rx+sizeof(code));
    CHECK(((int (*)(void))rx)() == 43);
    struct sigaction action = {.sa_sigaction = catch_fault, .sa_flags = SA_SIGINFO};
    sigemptyset(&action.sa_mask);
    CHECK(sigaction(SIGSEGV, &action, NULL) == 0 && sigaction(SIGBUS, &action, NULL) == 0);
    fault_address = rx; fault_signal = SIGSEGV; fault_code = SEGV_ACCERR;
    __asm__ volatile("strb %w1, [%0]" :: "r"(rx), "r"(255) : "memory");
    CHECK(faults == 1 && ((int (*)(void))rx)() == 43);
    fault_address = rw; execute_fault = 1;
    ((void (*)(void))rw)();
    CHECK(faults == 2);
    execute_fault = 0;
    CHECK(ftruncate(fd, page+17) == 0);
    CHECK(rw[page] == 0x58 && rx[page+17] == 0);
    CHECK(ftruncate(fd, page) == 0);
    fault_address = rx+page; fault_signal = SIGBUS; fault_code = BUS_ADRERR;
    __asm__ volatile("ldrb wzr, [%0]" :: "r"(rx+page) : "memory");
    CHECK(faults == 3);
    CHECK(ftruncate(fd, 3*page) == 0);
    // Growth repopulates old aliases on fault and keeps new data coherent.
    CHECK(rx[page] == 0 && rx[2*page] == 0);
    rw[2*page] = 0x79; CHECK(rx[2*page] == 0x79);
    CHECK(fcntl(fd, F_ADD_SEALS, F_SEAL_WRITE) == -1 && errno == EBUSY);
    CHECK(fcntl(fd, F_GET_SEALS) == 0);
    CHECK(fcntl(fd, F_ADD_SEALS, F_SEAL_FUTURE_WRITE|F_SEAL_SHRINK|F_SEAL_GROW) == 0);
    CHECK(pwrite(fd, "x", 1, 0) == -1 && errno == EPERM);
    CHECK(ftruncate(fd, page) == -1 && errno == EPERM);
    CHECK(ftruncate(fd, 4*page) == -1 && errno == EPERM);
    CHECK(mmap(NULL, page, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0) == MAP_FAILED && errno == EPERM);
    unsigned char *sealed = mmap(NULL, page, PROT_READ, MAP_SHARED, fd, 0);
    CHECK(sealed != MAP_FAILED && mprotect(sealed, page, PROT_READ|PROT_WRITE) == -1 && errno == EACCES);
    rw[101] = 0x7a; CHECK(sealed[101] == 0x7a && rx[101] == 0x7a);
    CHECK(munmap(rw, 3*page) == 0 && munmap(rx, 3*page) == 0);
    CHECK(fcntl(fd, F_ADD_SEALS, F_SEAL_WRITE|F_SEAL_SEAL) == 0);
    CHECK(fcntl(duplicate, F_GET_SEALS) == 31);
    CHECK(fcntl(fd, F_ADD_SEALS, 0) == -1 && errno == EPERM);
    CHECK(close(fd) == 0 && close(duplicate) == 0);
    int reused = new_file("reused-descriptor", 0);
    CHECK(reused == fd && pwrite(reused, "other", 5, 101) == 5);
    CHECK(sealed[101] == 0x7a && munmap(sealed, page) == 0);
    CHECK(close(reused) == 0);
    int fixed = new_file("fixed", 0);
    CHECK(fixed >= 0 && fcntl(fixed, F_GET_SEALS) == F_SEAL_SEAL);
    CHECK(fcntl(fixed, F_ADD_SEALS, F_SEAL_WRITE) == -1 && errno == EPERM);
    CHECK(pwrite(fixed, "end", 3, page+10) == 3);
    CHECK(lseek(fixed, 0, SEEK_END) == (off_t)(page+13));
    CHECK(pread(fixed, link, 1, page) == 1 && link[0] == 0);
    CHECK(close(fixed) == 0);
    // Exercise ART's actual 64 MiB layout and fixed R -> RX replacement.
    // The evidence gate also checks physical peak usage: these four views
    // must own one 64 MiB backing object, not duplicate physical copies.
    const size_t half = 32*1024*1024;
    int cache = new_file("jit-layout", 0);
    CHECK(cache >= 0 && ftruncate(cache, 2*half) == 0);
    unsigned char *code_write = mmap(NULL, half, PROT_READ|PROT_WRITE, MAP_SHARED, cache, half);
    unsigned char *data_write = mmap(NULL, half, PROT_READ|PROT_WRITE, MAP_SHARED, cache, 0);
    unsigned char *data_read = mmap((void *)(uintptr_t)0x90000000, 2*half, PROT_READ, MAP_SHARED, cache, 0);
    CHECK(code_write != MAP_FAILED && data_write != MAP_FAILED && data_read != MAP_FAILED);
    unsigned char *code_read = mmap(data_read+half, half, PROT_READ|PROT_EXEC, MAP_SHARED|MAP_FIXED, cache, half);
    CHECK(code_read == data_read+half);
    data_write[17] = 0x5b; CHECK(data_read[17] == 0x5b);
    memcpy(code_write, code, sizeof(code));
    __builtin___clear_cache((char *)code_write, (char *)code_write+sizeof(code));
    __builtin___clear_cache((char *)code_read, (char *)code_read+sizeof(code));
    CHECK(((int (*)(void))code_read)() == 42);
    CHECK(close(cache) == 0 && data_read[17] == 0x5b);
    CHECK(munmap(code_write, half) == 0 && munmap(data_write, half) == 0);
    CHECK(((int (*)(void))code_read)() == 42 && munmap(data_read, 2*half) == 0);
    puts("ART_MEMFD_OK aliases=1 code=1 truncate=1 seals=1 lifetime=1 wx=1");
    return 0;
}
