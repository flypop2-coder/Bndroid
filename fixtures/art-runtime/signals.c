#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/futex.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdatomic.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <time.h>
#include <ucontext.h>
#include <unistd.h>

_Static_assert(offsetof(ucontext_t, uc_mcontext) == 176, "AArch64 ucontext ABI");
_Static_assert(offsetof(ucontext_t, uc_mcontext.__reserved) == 464, "AArch64 FPSIMD ABI");
_Static_assert(sizeof(ucontext_t) == 4560, "AArch64 context size");
#define CHECK(x) do { if (!(x)) { fprintf(stderr,"ART_SIGNAL_FAILED line=%d errno=%d handler=%d\n",__LINE__,errno,atomic_load(&failure)); return 1; } } while (0)
#define H(x) do { if (!(x)) atomic_store(&failure,__LINE__); } while (0)
static _Atomic int failure, calls, ready, word, nesting;
static int main_tid, bad_frame, realtime, values[4], value_count, autodisarm;
static uintptr_t alternate;
static const size_t alternate_size=32768;
extern int signal_register_probe(int pid, int tid, int signal);

static void handler(int signal, siginfo_t *info, void *opaque) {
    int saved_errno=errno;
    ucontext_t *context=opaque;
    char on_stack;
    sigset_t mask;
    H(sigprocmask(SIG_SETMASK,NULL,&mask)==0);
    H(info->si_signo==signal);
    if (syscall(SYS_gettid)==main_tid) {
        H((uintptr_t)&on_stack>=alternate && (uintptr_t)&on_stack<alternate+alternate_size);
        stack_t stack;
        H(sigaltstack(NULL,&stack)==0 && (autodisarm ? stack.ss_flags==SS_DISABLE : (stack.ss_flags&SS_ONSTACK)!=0));
        stack_t disabled={.ss_flags=SS_DISABLE};
        if (autodisarm) H(sigaltstack(&disabled,NULL)==0);
        else H(sigaltstack(&disabled,NULL)==-1 && errno==EPERM);
    }
    if (signal==SIGSEGV || signal==SIGBUS) {
        H(info->si_code==(signal==SIGSEGV ? SEGV_MAPERR : BUS_ADRERR));
        context->uc_mcontext.pc+=4; // The fixture's single faulting load.
    } else {
        H(info->si_pid==getpid() && info->si_uid==getuid());
        if (signal==realtime && value_count<4) values[value_count++]=info->si_value.sival_int;
    }
    if (bad_frame==1) context->uc_mcontext.pstate=0x345; // Must never return to EL1.
    if (bad_frame==2) context->uc_mcontext.pc=0x40080000; // No kernel entry.
    if (bad_frame==3) ((uint32_t *)context->uc_mcontext.__reserved)[1]=16; // Invalid FPSIMD record.
    H(sigismember(&mask,SIGUSR2)==1);
    H(sigismember(&mask,signal)==(signal==SIGUSR1 && atomic_load(&nesting) ? 0 : 1));
    if (signal==SIGUSR1 && atomic_load(&nesting)==1) {
        atomic_store(&nesting,2);
        H(raise(SIGUSR1)==0);
        H(atomic_load(&nesting)==3);
    } else if (atomic_load(&nesting)==2) atomic_store(&nesting,3);
    // A handler-side futex must not consume its interrupted wait's restart.
    H(syscall(SYS_futex,&word,FUTEX_WAIT_PRIVATE,123,NULL,0,0)==-1 && errno==EAGAIN);
    __asm__ volatile("movi v8.16b, #0\nmsr fpcr, xzr\nmsr fpsr, xzr" ::: "v8","memory");
    atomic_fetch_add(&calls,1);
    errno=saved_errno;
}
static long long now_ns(void) {
    struct timespec t;
    if(clock_gettime(CLOCK_MONOTONIC,&t)) return -1;
    return (long long)t.tv_sec*1000000000LL+t.tv_nsec;
}
static int work(int kind) {
    atomic_store(&ready,1);
    if (kind==3) {
        struct timespec request={1,0}, remaining={0,0};
        CHECK(nanosleep(&request,&remaining)==-1 && errno==EINTR);
        CHECK(remaining.tv_sec==0 && remaining.tv_nsec>500000000 && remaining.tv_nsec<1000000000);
    } else if (kind==4) {
        sigset_t set; sigemptyset(&set); sigaddset(&set,realtime);
        siginfo_t info;
        CHECK(sigwaitinfo(&set,&info)==realtime && info.si_pid==getpid());
    } else if (kind==5) {
        CHECK(usleep(20000)==0);
        CHECK(syscall(SYS_tgkill,getpid(),main_tid,SIGUSR1)==0);
    } else {
        struct timespec timeout={0,200000000};
        long long started=now_ns();
        int result=syscall(SYS_futex,&word,FUTEX_WAIT_PRIVATE,0,kind==2 ? &timeout : NULL,0,0);
        if (kind==0) CHECK(result==-1 && errno==EINTR);
        if (kind==1) CHECK(result==0 && atomic_load(&word)==1);
        if (kind==2) {
            CHECK(result==-1 && errno==ETIMEDOUT);
            CHECK(now_ns()-started>=180000000 && now_ns()-started<300000000);
        }
    }
    return 0;
}
static void *worker(void *argument) { return (void *)(uintptr_t)work((int)(uintptr_t)argument); }
static int launch(pthread_t *thread,int kind) {
    atomic_store(&ready,0); atomic_store(&word,0);
    CHECK(pthread_create(thread,NULL,worker,(void *)(uintptr_t)kind)==0);
    while(!atomic_load(&ready)) sched_yield();
    return 0;
}
static int join(pthread_t thread) {
    void *result=(void *)1;
    CHECK(pthread_join(thread,&result)==0 && result==NULL && atomic_load(&failure)==0);
    return 0;
}
static _Atomic int requeue_ready, requeue_source, requeue_target;
static void *requeue_worker(void *unused) {
    (void)unused;
    atomic_fetch_add(&requeue_ready,1);
    int result=syscall(SYS_futex,&requeue_source,FUTEX_WAIT_PRIVATE,0,NULL,0,0);
    return (void *)(uintptr_t)(result!=0);
}
static struct timespec after(struct timespec value,long nanos) {
    value.tv_nsec+=nanos;
    value.tv_sec+=value.tv_nsec/1000000000;
    value.tv_nsec%=1000000000;
    return value;
}
static int synchronization(void) {
    for(int compare=0;compare<2;++compare) {
        pthread_t workers[3]; atomic_store(&requeue_ready,0);
        for(int n=0;n<3;++n) CHECK(pthread_create(&workers[n],NULL,requeue_worker,NULL)==0);
        while(atomic_load(&requeue_ready)!=3) sched_yield();
        CHECK(usleep(20000)==0);
        CHECK(syscall(SYS_futex,&requeue_source,FUTEX_CMP_REQUEUE_PRIVATE,1,2,&requeue_target,1)==-1 && errno==EAGAIN);
        CHECK(syscall(SYS_futex,&requeue_source,FUTEX_REQUEUE_PRIVATE,-1,2,&requeue_target,0)==-1 && errno==EINVAL);
        CHECK(syscall(SYS_futex,&requeue_source,FUTEX_REQUEUE_PRIVATE,1,2,(void *)0x40080000,0)==-1 && errno==EFAULT);
        CHECK(syscall(SYS_futex,&requeue_source,compare ? FUTEX_CMP_REQUEUE_PRIVATE : FUTEX_REQUEUE_PRIVATE,1,2,&requeue_target,0)==3);
        CHECK(syscall(SYS_futex,&requeue_source,FUTEX_WAKE_PRIVATE,3,NULL,0,0)==0);
        CHECK(syscall(SYS_futex,&requeue_target,FUTEX_WAKE_PRIVATE,3,NULL,0,0)==2);
        for(int n=0;n<3;++n) CHECK(join(workers[n])==0);
    }
    struct timespec real, coarse, end, later;
    CHECK(clock_gettime(CLOCK_REALTIME,&real)==0 && real.tv_sec>1600000000 && real.tv_nsec>=0 && real.tv_nsec<1000000000);
    CHECK(clock_gettime(CLOCK_REALTIME_COARSE,&coarse)==0 && coarse.tv_sec>=real.tv_sec-1 && coarse.tv_sec<=real.tv_sec+1);
    end=after(real,30000000); long long started=now_ns();
    CHECK(syscall(SYS_futex,&requeue_source,FUTEX_WAIT_BITSET_PRIVATE|FUTEX_CLOCK_REALTIME,0,&end,0,FUTEX_BITSET_MATCH_ANY)==-1 && errno==ETIMEDOUT);
    CHECK(now_ns()-started>=25000000 && now_ns()-started<100000000);
    CHECK(clock_gettime(CLOCK_REALTIME,&later)==0);
    long long advanced=(later.tv_sec-real.tv_sec)*1000000000LL+later.tv_nsec-real.tv_nsec;
    CHECK(advanced>=25000000 && advanced<100000000);
    end=after(later,20000000); started=now_ns();
    CHECK(clock_nanosleep(CLOCK_REALTIME,TIMER_ABSTIME,&end,NULL)==0);
    CHECK(now_ns()-started>=15000000 && now_ns()-started<100000000);
    puts("ART_SYNC_OK requeue=1 realtime_clock=1 absolute_wait=1");
    return 0;
}
int main(int argc,char **argv) {
    main_tid=syscall(SYS_gettid); realtime=SIGRTMIN+4;
    alternate=(uintptr_t)mmap(NULL,alternate_size,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    CHECK(alternate!=(uintptr_t)MAP_FAILED);
    stack_t stack={.ss_sp=(void *)alternate,.ss_size=alternate_size}, oldstack;
    CHECK(sigaltstack(&stack,&oldstack)==0);
    struct sigaction action={.sa_sigaction=handler,.sa_flags=SA_SIGINFO|SA_ONSTACK}, old;
    sigemptyset(&action.sa_mask); sigaddset(&action.sa_mask,SIGUSR2);
    CHECK(sigaction(SIGUSR1,&action,&old)==0);
    CHECK(sigaction(SIGUSR1,NULL,&old)==0 && old.sa_sigaction==handler && (old.sa_flags&SA_ONSTACK));
    if (argc>1 && !strncmp(argv[1],"signal-bad-",11)) {
        bad_frame=!strcmp(argv[1],"signal-bad-pc") ? 2 : !strcmp(argv[1],"signal-bad-fp") ? 3 : 1;
        CHECK(write(1,"SIGNAL_BAD_FRAME_ARMED\n",23)==23); raise(SIGUSR1);
        CHECK(0);
    }
    if (argc>1 && !strcmp(argv[1],"signal-guard")) {
        CHECK(mprotect((void *)alternate,alternate_size,PROT_NONE)==0);
        CHECK(write(1,"SIGNAL_GUARD_ARMED\n",19)==19); raise(SIGUSR1); CHECK(0);
    }
    if (argc>1 && !strcmp(argv[1],"signal-default")) {
        sigset_t blocked; sigemptyset(&blocked); sigaddset(&blocked,realtime);
        CHECK(sigprocmask(SIG_BLOCK,&blocked,NULL)==0);
        pthread_t t; CHECK(launch(&t,0)==0); CHECK(usleep(20000)==0);
        for(int n=0;n<32;++n) { union sigval value={.sival_int=n}; CHECK(sigqueue(getpid(),realtime,value)==0); }
        union sigval value={.sival_int=32}; CHECK(sigqueue(getpid(),realtime,value)==-1 && errno==EAGAIN);
        CHECK(write(1,"SIGNAL_DEFAULT_ARMED\n",21)==21); kill(getpid(),SIGKILL); CHECK(0);
    }
    CHECK(sigaction(SIGKILL,&action,NULL)==-1 && errno==EINVAL);
    CHECK(syscall(SYS_tkill,0,SIGUSR1)==-1 && errno==EINVAL);
    CHECK(syscall(SYS_tgkill,getpid(),0,SIGUSR1)==-1 && errno==EINVAL);
    CHECK(syscall(SYS_rt_sigaction,SIGUSR1,NULL,NULL,16)==-1 && errno==EINVAL);
    CHECK(syscall(SYS_rt_sigaction,SIGUSR1,(void *)0x40080000,NULL,8)==-1 && errno==EFAULT);
    CHECK(signal_register_probe(getpid(),main_tid,SIGUSR1)==0 && atomic_load(&calls)==1 && !atomic_load(&failure));
    sigset_t set,previous,pending;
    sigemptyset(&set); sigaddset(&set,SIGUSR1);
    CHECK(sigprocmask(SIG_BLOCK,&set,&previous)==0);
    CHECK(kill(getpid(),SIGUSR1)==0 && kill(getpid(),SIGUSR1)==0);
    CHECK(sigpending(&pending)==0 && sigismember(&pending,SIGUSR1)==1 && atomic_load(&calls)==1);
    CHECK(sigprocmask(SIG_SETMASK,&previous,NULL)==0 && atomic_load(&calls)==2);
    action.sa_flags|=SA_NODEFER; CHECK(sigaction(SIGUSR1,&action,NULL)==0);
    atomic_store(&nesting,1); CHECK(raise(SIGUSR1)==0 && atomic_load(&nesting)==3);
    atomic_store(&nesting,0); action.sa_flags&=~SA_NODEFER;
    CHECK(sigaction(SIGUSR1,&action,NULL)==0);
    CHECK(sigaction(realtime,&action,NULL)==0);
    sigemptyset(&set); sigaddset(&set,realtime); CHECK(sigprocmask(SIG_BLOCK,&set,&previous)==0);
    siginfo_t info={.si_signo=realtime,.si_code=SI_QUEUE};
    for(int n=10;n<12;++n) { union sigval value={.sival_int=n}; CHECK(sigqueue(getpid(),realtime,value)==0); }
    CHECK(sigprocmask(SIG_SETMASK,&previous,NULL)==0 && value_count==2 && values[0]==10 && values[1]==11);
    CHECK(sigaction(SIGSEGV,&action,NULL)==0 && sigaction(SIGBUS,&action,NULL)==0);
    __asm__ volatile("ldr xzr, [%0]" :: "r"((uintptr_t)0) : "memory");
    int fd=open("/apex/com.android.art/javalib/core-oj.jar",O_RDONLY); struct stat file;
    CHECK(fd>=0 && fstat(fd,&file)==0);
    void *beyond=mmap(NULL,4096,PROT_READ,MAP_PRIVATE,fd,(file.st_size+4095)&~4095LL);
    CHECK(beyond!=MAP_FAILED);
    __asm__ volatile("ldr xzr, [%0]" :: "r"(beyond) : "memory");
    CHECK(munmap(beyond,4096)==0 && close(fd)==0 && !atomic_load(&failure));
    pthread_t t;
    CHECK(launch(&t,0)==0 && usleep(20000)==0 && pthread_kill(t,SIGUSR1)==0 && join(t)==0);
    action.sa_flags|=SA_RESTART; CHECK(sigaction(SIGUSR1,&action,NULL)==0);
    CHECK(launch(&t,1)==0 && usleep(20000)==0 && pthread_kill(t,SIGUSR1)==0);
    CHECK(usleep(20000)==0); atomic_store(&word,1);
    CHECK(syscall(SYS_futex,&word,FUTEX_WAKE_PRIVATE,1,NULL,0,0)==1 && join(t)==0);
    CHECK(launch(&t,2)==0 && usleep(150000)==0 && pthread_kill(t,SIGUSR1)==0 && join(t)==0);
    CHECK(launch(&t,3)==0 && usleep(20000)==0 && pthread_kill(t,SIGUSR1)==0 && join(t)==0);
    sigemptyset(&set); sigaddset(&set,realtime); CHECK(sigprocmask(SIG_BLOCK,&set,&previous)==0);
    CHECK(launch(&t,4)==0 && usleep(20000)==0 && pthread_kill(t,realtime)==0 && join(t)==0);
    struct timespec timeout={0,10000000};
    CHECK(sigtimedwait(&set,&info,&timeout)==-1 && errno==EAGAIN);
    CHECK(sigprocmask(SIG_SETMASK,&previous,NULL)==0);
    sigemptyset(&set); sigaddset(&set,SIGUSR1); CHECK(sigprocmask(SIG_BLOCK,&set,&previous)==0);
    CHECK(launch(&t,5)==0);
    sigset_t empty; sigemptyset(&empty); CHECK(sigsuspend(&empty)==-1 && errno==EINTR && join(t)==0);
    CHECK(sigprocmask(SIG_SETMASK,NULL,&pending)==0 && sigismember(&pending,SIGUSR1)==1);
    CHECK(sigprocmask(SIG_SETMASK,&previous,NULL)==0);
    action.sa_flags|=SA_RESETHAND; CHECK(sigaction(SIGUSR2,&action,NULL)==0 && raise(SIGUSR2)==0);
    CHECK(sigaction(SIGUSR2,NULL,&old)==0 && old.sa_handler==SIG_DFL);
    autodisarm=1; stack.ss_flags=(int)0x80000000U;
    CHECK(sigaltstack(&stack,NULL)==0 && raise(SIGUSR1)==0);
    CHECK(sigaltstack(NULL,&oldstack)==0 && (unsigned)oldstack.ss_flags==0x80000000U);
    autodisarm=0;
    CHECK(synchronization()==0);
    struct iovec message[2]={{.iov_base="ART_SIGNAL_STDERR",.iov_len=17},{.iov_base="_OK\n",.iov_len=4}};
    CHECK(writev(2,message,2)==21);
    stack.ss_flags=SS_DISABLE; CHECK(sigaltstack(&stack,NULL)==0 && munmap((void *)alternate,alternate_size)==0);
    CHECK(!atomic_load(&failure));
    puts("ART_SIGNALS_OK handlers=1 altstack=1 registers=1 masks=1 realtime=1 faults=1 threads=1 restart=1 waits=1");
    return 0;
}
