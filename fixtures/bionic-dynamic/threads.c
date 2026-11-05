#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/futex.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { dprintf(2, "THREAD_CHECK_FAILED line=%d errno=%d\n", __LINE__, errno); _exit(1); } } while (0)
#define WORKERS 4
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_barrier_t barrier;
static pthread_key_t key;
static _Atomic int destructors;
static volatile _Thread_local int local = 19;
static pid_t tids[WORKERS], main_pid;
static int counter, shared_fd;
static int (*plugin_compute)(int);
static _Atomic int detached_done;

static void *detached_worker(void *argument) {
    (void)argument;
    CHECK(local == 19);
    local = 88;
    CHECK(pthread_setspecific(key, malloc(32)) == 0 && pthread_getspecific(key) != NULL);
    __asm__ volatile("wfi" ::: "memory");
    CHECK(local == 88);
    atomic_fetch_add(&detached_done, 1);
    return NULL;
}

static void delay(void) {
    struct timespec pause = {0, 10000000};
    CHECK(nanosleep(&pause, NULL) == 0);
}

#if BNDROID_THREAD_TEST_MODE != 0
static _Atomic int life_ready, life_release;
static int life_word;
static void *lifecycle_waiter(void *argument) {
    (void)argument;
    atomic_store(&life_ready, 1);
    CHECK(syscall(SYS_futex, &life_word, FUTEX_WAIT_PRIVATE, 0, NULL, NULL, 0) == 0);
    dprintf(2, "THREAD_LIFECYCLE_WAIT_RETURNED\n");
    _exit(2);
}
static void *lifecycle_actor(void *argument) {
    (void)argument;
    while (!atomic_load(&life_release)) delay();
    delay();
#if BNDROID_THREAD_TEST_MODE == 1
    dprintf(1, "THREAD_LEADER_EXIT_LAST_WORKER_OK\n");
    return NULL;
#elif BNDROID_THREAD_TEST_MODE == 2
    dprintf(1, "THREAD_GROUP_EXIT_ARMED\n");
    syscall(SYS_exit_group, 0);
#elif BNDROID_THREAD_TEST_MODE == 3
    volatile uintptr_t bad = 0x40080000;
    dprintf(1, "THREAD_WORKER_FAULT_ARMED address=0x40080000\n");
    volatile unsigned char denied = *(unsigned char *)bad;
    (void)denied;
#endif
    dprintf(2, "THREAD_LIFECYCLE_MISSED\n");
    _exit(2);
}
static void lifecycle(void) {
    pthread_t actor, waiter;
#if BNDROID_THREAD_TEST_MODE != 1
    CHECK(pthread_create(&waiter, NULL, lifecycle_waiter, NULL) == 0);
    while (!atomic_load(&life_ready)) delay();
    delay();
#else
    (void)waiter;
    (void)lifecycle_waiter;
#endif
    CHECK(pthread_create(&actor, NULL, lifecycle_actor, NULL) == 0);
    atomic_store(&life_release, 1);
#if BNDROID_THREAD_TEST_MODE == 1
    dprintf(1, "THREAD_LEADER_EXIT_ARMED\n");
    pthread_exit(NULL);
#else
    for (;;) delay();
#endif
}
#endif

static void destroy_value(void *value) {
    CHECK(value != NULL);
    free(value);
    atomic_fetch_add(&destructors, 1);
}
static void *worker(void *argument) {
    int index = (int)(intptr_t)argument;
    tids[index] = (pid_t)syscall(SYS_gettid);
    CHECK(getpid() == main_pid && tids[index] != main_pid && gettid() == tids[index]);
    CHECK(local == 19 && plugin_compute(6) == 42);
    local = 100 + index;
    errno = 70 + index;
    for (int tick = 0; tick < 3; ++tick) __asm__ volatile("wfi" ::: "memory");
    CHECK(local == 100 + index && errno == 70 + index && plugin_compute(6) == 43);
    CHECK(pthread_setspecific(key, malloc(24)) == 0 && pthread_getspecific(key) != NULL);
    CHECK(pthread_setname_np(pthread_self(), "bndroid-worker") == 0);
    char name[16];
    CHECK(prctl(PR_GET_NAME, name) == 0 && strcmp(name, "bndroid-worker") == 0);
    sigset_t mask;
    CHECK(pthread_sigmask(SIG_SETMASK, NULL, &mask) == 0 && sigismember(&mask, SIGUSR1) == 1);
    sigemptyset(&mask);
    sigaddset(&mask, SIGUSR1);
    CHECK(pthread_sigmask(SIG_UNBLOCK, &mask, NULL) == 0);
    int result = pthread_barrier_wait(&barrier);
    CHECK(result == 0 || result == PTHREAD_BARRIER_SERIAL_THREAD);
    for (int i = 0; i < 300; ++i) {
        CHECK(pthread_mutex_lock(&mutex) == 0);
        ++counter;
        if (i == 0) {
            char bytes[4];
            CHECK(read(shared_fd, bytes, sizeof(bytes)) == sizeof(bytes));
            // Hold the mutex while descheduled to force real futex contention.
            struct timespec delay = {0, 1000000};
            CHECK(nanosleep(&delay, NULL) == 0);
        }
        CHECK(pthread_mutex_unlock(&mutex) == 0);
        if (i % 25 == 0) CHECK(sched_yield() == 0);
    }
    CHECK(local == 100 + index);
    return (void *)(intptr_t)(index + 30);
}

void test_threads(void) {
#if BNDROID_THREAD_TEST_MODE != 0
    lifecycle();
#endif
    main_pid = getpid();
    CHECK(syscall(SYS_gettid) == main_pid);
    sigset_t mask, saved;
    sigemptyset(&mask);
    sigaddset(&mask, SIGUSR1);
    CHECK(pthread_sigmask(SIG_BLOCK, &mask, &saved) == 0);
    CHECK(pthread_key_create(&key, destroy_value) == 0);
    CHECK(pthread_barrier_init(&barrier, NULL, WORKERS + 1) == 0);
    shared_fd = open("/system/etc/runtime-data", O_RDONLY);
    CHECK(shared_fd >= 3);
    void *plugin = dlopen("libbndroidplugin.so", RTLD_NOW | RTLD_LOCAL);
    CHECK(plugin != NULL);
    plugin_compute = (int (*)(int))dlsym(plugin, "bndroid_plugin_compute");
    CHECK(plugin_compute != NULL);
    pthread_t threads[WORKERS];
    for (int i = 0; i < WORKERS; ++i) {
        int error = pthread_create(&threads[i], NULL, worker, (void *)(intptr_t)i);
        if (error) dprintf(2, "THREAD_CREATE_ERROR error=%d\n", error);
        CHECK(error == 0);
    }
    int result = pthread_barrier_wait(&barrier);
    CHECK(result == 0 || result == PTHREAD_BARRIER_SERIAL_THREAD);
    for (int i = 0; i < WORKERS; ++i) {
        void *returned;
        CHECK(pthread_join(threads[i], &returned) == 0 && returned == (void *)(intptr_t)(i + 30));
        for (int j = 0; j < i; ++j) CHECK(tids[i] != tids[j]);
    }
    CHECK(counter == WORKERS * 300 && atomic_load(&destructors) == WORKERS && local == 19);
    CHECK(lseek(shared_fd, 0, SEEK_CUR) == WORKERS * 4 && close(shared_fd) == 0);
    CHECK(pthread_sigmask(SIG_SETMASK, NULL, &mask) == 0 && sigismember(&mask, SIGUSR1) == 1);
    CHECK(pthread_sigmask(SIG_SETMASK, &saved, NULL) == 0);
    CHECK(pthread_barrier_destroy(&barrier) == 0);
    CHECK(dlclose(plugin) == 0);
    // Recycle joinable and detached scheduler slots. Detached Bionic tears
    // down its own user stack before issuing the final exit syscall.
    pthread_attr_t detached_attributes;
    CHECK(pthread_attr_init(&detached_attributes) == 0);
    CHECK(pthread_attr_setdetachstate(&detached_attributes, PTHREAD_CREATE_DETACHED) == 0);
    for (int iteration = 0; iteration < 8; ++iteration) {
        pthread_t detached;
        CHECK(pthread_create(&detached, &detached_attributes, detached_worker, NULL) == 0);
        for (int spin = 0; atomic_load(&detached_done) <= iteration && spin < 100; ++spin) delay();
        CHECK(atomic_load(&detached_done) == iteration + 1);
        delay();
    }
    CHECK(atomic_load(&destructors) == WORKERS + 8);
    CHECK(pthread_attr_destroy(&detached_attributes) == 0 && pthread_key_delete(key) == 0);

    pthread_condattr_t attributes;
    pthread_cond_t condition;
    CHECK(pthread_condattr_init(&attributes) == 0);
    CHECK(pthread_condattr_setclock(&attributes, CLOCK_MONOTONIC) == 0);
    CHECK(pthread_cond_init(&condition, &attributes) == 0);
    struct timespec until;
    CHECK(clock_gettime(CLOCK_MONOTONIC, &until) == 0);
    until.tv_nsec += 10000000;
    if (until.tv_nsec >= 1000000000) { ++until.tv_sec; until.tv_nsec -= 1000000000; }
    CHECK(pthread_mutex_lock(&mutex) == 0);
    CHECK(pthread_cond_timedwait(&condition, &mutex, &until) == ETIMEDOUT);
    struct timespec after;
    CHECK(clock_gettime(CLOCK_MONOTONIC, &after) == 0);
    CHECK(after.tv_sec > until.tv_sec || (after.tv_sec == until.tv_sec && after.tv_nsec >= until.tv_nsec));
    CHECK(pthread_mutex_unlock(&mutex) == 0);
    CHECK(pthread_cond_destroy(&condition) == 0 && pthread_condattr_destroy(&attributes) == 0);
    int word = 7;
    struct timespec zero = {0, 0};
    CHECK(syscall(SYS_futex, &word, FUTEX_WAIT_PRIVATE, 8, NULL, NULL, 0) == -1 && errno == EAGAIN);
    CHECK(syscall(SYS_futex, &word, FUTEX_WAIT_PRIVATE, 7, &zero, NULL, 0) == -1 && errno == ETIMEDOUT);
    CHECK(syscall(SYS_futex, (char *)&word + 1, FUTEX_WAIT_PRIVATE, 7, NULL, NULL, 0) == -1 && errno == EINVAL);
    CHECK(syscall(SYS_futex, (void *)0x40080000, FUTEX_WAIT_PRIVATE, 7, NULL, NULL, 0) == -1 && errno == EFAULT);
    CHECK(syscall(SYS_futex, &word, FUTEX_WAIT_BITSET_PRIVATE, 7, NULL, NULL, 0) == -1 && errno == EINVAL);
    void *stack = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    CHECK(stack != MAP_FAILED);
    CHECK(syscall(SYS_clone, 0x3d0f00 | SIGCHLD, (char *)stack + 4096, NULL, NULL, NULL) == -1 && errno == EINVAL);
    CHECK(syscall(SYS_clone, 0x3d0f00, (char *)stack + 4096, (void *)0x40080000, NULL, NULL) == -1 && errno == EFAULT);
    CHECK(munmap(stack, 4096) == 0);
    puts("DYNAMIC_THREADS_OK threads=12 tls=1 mutex=1 barrier=1 join=1 detached=1 timed_wait=1 shared_files=1 destructors=1");
}
