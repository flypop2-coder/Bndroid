#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <time.h>
#include <unistd.h>

#define CHECK(x) do {if (!(x)) {fprintf(stderr,"POLL_CHECK_FAILED pid=%d line=%d errno=%d\n",getpid(),__LINE__,errno);_exit(92);}} while(0)
static _Atomic int phase, completed;
static volatile sig_atomic_t handled;
static int shared_fd, operation;
static uint64_t *read_destination;
static void handler(int sig) {if(sig!=SIGUSR1)_exit(93);++handled;}
static uint64_t now(void) {struct timespec t;CHECK(clock_gettime(CLOCK_MONOTONIC,&t)==0);return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
static void write_value(int fd,uint64_t value) {CHECK(write(fd,&value,8)==8);}
static uint64_t read_value(int fd) {uint64_t v=0;CHECK(read(fd,&v,8)==8);return v;}
static void watch(int ep,int op,int fd,uint32_t mask,uint64_t data) {struct epoll_event e={.events=mask,.data.u64=data};CHECK(epoll_ctl(ep,op,fd,op==EPOLL_CTL_DEL?NULL:&e)==0);}
static void wait_phase(int value) {while(atomic_load(&phase)<value)usleep(1000);usleep(30000);}
static void *worker(void *unused) {
    (void)unused;atomic_fetch_add(&phase,1);
    uint64_t value=1;struct epoll_event e;
    switch(operation) {
        case 1: CHECK(read_value(shared_fd)==42);break;
        case 2: CHECK(write(shared_fd,&value,8)==8);break;
        case 3: CHECK(read_value(shared_fd)==1);break;
        case 4: CHECK(epoll_wait(shared_fd,&e,1,1000)==1 && e.events==EPOLLIN && e.data.u64==0x123456789abcdef0ULL);break;
        case 5: CHECK(read(shared_fd,&value,8)==-1 && errno==EINTR);break;
        case 6: CHECK(read_value(shared_fd)==73);break;
        case 7: CHECK(read(shared_fd,read_destination,8)==-1 && errno==EFAULT);break;
        case 8: CHECK(epoll_wait(shared_fd,&e,1,-1)==-1 && errno==EINTR);break;
        case 9: (void)epoll_wait(shared_fd,&e,1,-1);_exit(94);
        default:_exit(95);
    }
    atomic_fetch_add(&completed,1);return NULL;
}
static pthread_t launch(int fd,int op) {phase=0;completed=0;shared_fd=fd;operation=op;pthread_t thread;CHECK(pthread_create(&thread,NULL,worker,NULL)==0);wait_phase(1);CHECK(completed==0);return thread;}
static void join(pthread_t thread) {CHECK(pthread_join(thread,NULL)==0 && completed==1);}
static void counters(void) {
    CHECK(eventfd(0,4)==-1 && errno==EINVAL);
    int fd=eventfd(0,EFD_NONBLOCK|EFD_CLOEXEC);CHECK(fd>=0);
    CHECK(fcntl(fd,F_GETFD)==FD_CLOEXEC && (fcntl(fd,F_GETFL)&O_NONBLOCK));
    uint64_t v=0;CHECK(read(fd,&v,8)==-1 && errno==EAGAIN);
    CHECK(read(fd,&v,7)==-1 && errno==EINVAL);CHECK(write(fd,&v,7)==-1 && errno==EINVAL);
    v=UINT64_MAX;CHECK(write(fd,&v,8)==-1 && errno==EINVAL);
    write_value(fd,5);write_value(fd,7);CHECK(read_value(fd)==12);
    write_value(fd,UINT64_MAX-1);v=1;CHECK(write(fd,&v,8)==-1 && errno==EAGAIN);CHECK(read_value(fd)==UINT64_MAX-1);
    write_value(fd,0);CHECK(read(fd,&v,8)==-1 && errno==EAGAIN);
    unsigned char bytes[8],copy[8];v=0x102030405060708ULL;memcpy(bytes,&v,8);
    struct iovec out[2]={{bytes,3},{bytes+3,5}},in[2]={{copy,5},{copy+5,3}};
    CHECK(writev(fd,out,2)==8 && readv(fd,in,2)==8 && memcmp(bytes,copy,8)==0);
    int alias=dup(fd);CHECK(alias>=0 && fcntl(alias,F_SETFD,0)==0 && fcntl(fd,F_GETFD)==FD_CLOEXEC);
    CHECK(fcntl(alias,F_SETFL,0)==0 && !(fcntl(fd,F_GETFL)&O_NONBLOCK));
    write_value(alias,9);uint64_t two[2]={0,0x55};CHECK(read(fd,two,sizeof(two))==8 && two[0]==9 && two[1]==0x55);
    CHECK(close(alias)==0 && close(fd)==0);
    fd=eventfd(3,EFD_SEMAPHORE|EFD_NONBLOCK);CHECK(fd>=0);
    for(int i=0;i<3;++i)CHECK(read_value(fd)==1);
    CHECK(read(fd,&v,8)==-1 && errno==EAGAIN && close(fd)==0);
}
static void epoll_modes(void) {
    CHECK(sizeof(struct epoll_event)==16 && epoll_create1(1)==-1 && errno==EINVAL);
    int ep=epoll_create1(EPOLL_CLOEXEC),a=eventfd(0,EFD_NONBLOCK),b=eventfd(0,EFD_NONBLOCK);CHECK(ep>=0 && a>=0 && b>=0);
    CHECK(fcntl(ep,F_GETFD)==FD_CLOEXEC);
    struct epoll_event event={.events=EPOLLIN,.data.u64=1},result[2];
    CHECK(epoll_ctl(ep,EPOLL_CTL_ADD,ep,&event)==-1 && errno==EINVAL);
    int plain=open("/dev/null",O_RDONLY);CHECK(plain>=0);
    CHECK(epoll_ctl(ep,EPOLL_CTL_ADD,plain,&event)==-1 && errno==EPERM && close(plain)==0);
    CHECK(epoll_ctl(ep,EPOLL_CTL_ADD,a,(void *)(uintptr_t)0x40080000)==-1 && errno==EFAULT);
    watch(ep,EPOLL_CTL_ADD,a,EPOLLIN,0x123456789abcdef0ULL);
    CHECK(epoll_ctl(ep,EPOLL_CTL_ADD,a,&event)==-1 && errno==EEXIST);
    CHECK(epoll_wait(ep,result,0,0)==-1 && errno==EINVAL);
    CHECK(epoll_wait(ep,result,2,0)==0);write_value(a,1);
    for(int i=0;i<2;++i)CHECK(epoll_wait(ep,result,2,0)==1 && result[0].events==EPOLLIN && result[0].data.u64==0x123456789abcdef0ULL);
    watch(ep,EPOLL_CTL_MOD,a,EPOLLIN|EPOLLET,2);
    CHECK(epoll_wait(ep,result,2,0)==1 && result[0].data.u64==2);
    CHECK(epoll_wait(ep,result,2,0)==0);write_value(a,1);
    CHECK(epoll_wait(ep,result,2,0)==1 && read_value(a)==2 && epoll_wait(ep,result,2,0)==0);
    watch(ep,EPOLL_CTL_MOD,a,EPOLLIN|EPOLLONESHOT,3);write_value(a,1);
    CHECK(epoll_wait(ep,(void *)(uintptr_t)0x40080000,1,0)==-1 && errno==EFAULT);
    CHECK(epoll_wait(ep,result,2,0)==1 && result[0].data.u64==3 && epoll_wait(ep,result,2,0)==0);
    watch(ep,EPOLL_CTL_MOD,a,EPOLLIN|EPOLLONESHOT,4);
    CHECK(epoll_wait(ep,result,1,0)==1 && result[0].data.u64==4);
    watch(ep,EPOLL_CTL_MOD,a,EPOLLIN,10);watch(ep,EPOLL_CTL_ADD,b,EPOLLIN,20);write_value(b,1);
    CHECK(epoll_wait(ep,result,1,0)==1);uint64_t first=result[0].data.u64;
    CHECK(epoll_wait(ep,result,1,0)==1 && result[0].data.u64!=first);
    CHECK(read_value(a)==1 && read_value(b)==1);
    uint64_t start=now();CHECK(epoll_wait(ep,result,2,20)==0 && now()-start>=20000000ULL);
    struct timespec delay={.tv_nsec=15000000};start=now();
    CHECK(syscall(__NR_epoll_pwait2,ep,result,2,&delay,NULL,8)==0 && now()-start>=15000000ULL);
    delay.tv_nsec=1000000000;CHECK(syscall(__NR_epoll_pwait2,ep,result,2,&delay,NULL,8)==-1 && errno==EINVAL);
    sigset_t empty;sigemptyset(&empty);
    CHECK(syscall(__NR_epoll_pwait,ep,result,2,0,&empty,4)==-1 && errno==EINVAL);
    watch(ep,EPOLL_CTL_DEL,a,0,0);watch(ep,EPOLL_CTL_DEL,b,0,0);
    CHECK(close(a)==0 && close(b)==0 && close(ep)==0);
}
static void lifetimes(void) {
    int ep=epoll_create1(0),fd=eventfd(0,0),alias=dup(fd);CHECK(ep>=0 && fd>=0 && alias>=0);
    watch(ep,EPOLL_CTL_ADD,fd,EPOLLIN,11);CHECK(close(fd)==0);
    int reused=eventfd(0,EFD_NONBLOCK);CHECK(reused==fd);
    struct epoll_event event={.events=EPOLLIN},result;
    CHECK(epoll_ctl(ep,EPOLL_CTL_MOD,reused,&event)==-1 && errno==ENOENT);
    write_value(reused,9);CHECK(epoll_wait(ep,&result,1,0)==0);
    write_value(alias,7);CHECK(epoll_wait(ep,&result,1,0)==1 && result.data.u64==11);
    CHECK(close(alias)==0 && epoll_wait(ep,&result,1,0)==0);
    watch(ep,EPOLL_CTL_ADD,reused,EPOLLIN,12);CHECK(epoll_wait(ep,&result,1,0)==1 && result.data.u64==12);
    CHECK(close(ep)==0 && close(reused)==0);
    fd=eventfd(0,0);alias=dup(fd);CHECK(fd>=0 && alias>=0);
    pthread_t thread=launch(fd,1);CHECK(close(fd)==0);reused=eventfd(0,EFD_NONBLOCK);CHECK(reused==fd);
    write_value(alias,42);join(thread);CHECK(close(alias)==0 && close(reused)==0);
    fd=eventfd(0,0);CHECK(fd>=0);write_value(fd,UINT64_MAX-1);
    thread=launch(fd,2);CHECK(read_value(fd)==UINT64_MAX-1);join(thread);CHECK(read_value(fd)==1 && close(fd)==0);
    ep=epoll_create1(0);fd=eventfd(0,0);CHECK(ep>=0 && fd>=0);
    watch(ep,EPOLL_CTL_ADD,fd,EPOLLIN,0x123456789abcdef0ULL);
    thread=launch(ep,4);CHECK(close(ep)==0);reused=eventfd(0,EFD_NONBLOCK);CHECK(reused==ep);
    write_value(fd,1);join(thread);CHECK(close(fd)==0 && close(reused)==0);
}
static void contention(void) {
    int fd=eventfd(0,EFD_SEMAPHORE);CHECK(fd>=0);
    phase=0;completed=0;operation=3;shared_fd=fd;pthread_t one,two;
    CHECK(pthread_create(&one,NULL,worker,NULL)==0 && pthread_create(&two,NULL,worker,NULL)==0);
    wait_phase(2);CHECK(completed==0);write_value(fd,1);usleep(50000);CHECK(completed==1);
    write_value(fd,1);CHECK(pthread_join(one,NULL)==0 && pthread_join(two,NULL)==0 && completed==2 && close(fd)==0);
}
static void signals_and_copies(void) {
    struct sigaction action={.sa_handler=handler};sigemptyset(&action.sa_mask);CHECK(sigaction(SIGUSR1,&action,NULL)==0);
    int fd=eventfd(0,0);CHECK(fd>=0);pthread_t thread=launch(fd,5);
    CHECK(pthread_kill(thread,SIGUSR1)==0);join(thread);CHECK(handled==1);
    action.sa_flags=SA_RESTART;CHECK(sigaction(SIGUSR1,&action,NULL)==0);
    thread=launch(fd,6);CHECK(pthread_kill(thread,SIGUSR1)==0);usleep(40000);CHECK(handled==2 && completed==0);
    write_value(fd,73);join(thread);
    read_destination=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);CHECK(read_destination!=MAP_FAILED);
    thread=launch(fd,7);CHECK(munmap(read_destination,4096)==0);write_value(fd,1);join(thread);CHECK(close(fd)==0);
    int ep=epoll_create1(0);CHECK(ep>=0);thread=launch(ep,8);
    CHECK(pthread_kill(thread,SIGUSR1)==0);join(thread);CHECK(handled==3);
    sigset_t mask,original,empty,after;sigemptyset(&mask);sigaddset(&mask,SIGUSR1);sigemptyset(&empty);
    CHECK(pthread_sigmask(SIG_BLOCK,&mask,&original)==0 && pthread_kill(pthread_self(),SIGUSR1)==0 && handled==3);
    struct epoll_event event;
    CHECK(epoll_pwait(ep,&event,1,1000,&empty)==-1 && errno==EINTR && handled==4);
    CHECK(pthread_sigmask(SIG_SETMASK,NULL,&after)==0 && sigismember(&after,SIGUSR1)==1);
    CHECK(epoll_pwait(ep,&event,1,0,&empty)==0 && pthread_sigmask(SIG_SETMASK,NULL,&after)==0 && sigismember(&after,SIGUSR1)==1);
    CHECK(pthread_sigmask(SIG_SETMASK,&original,NULL)==0 && close(ep)==0);
}
static void nested(void) {
    int child=epoll_create1(0),parent=epoll_create1(0),fd=eventfd(0,0);CHECK(child>=0 && parent>=0 && fd>=0);
    watch(child,EPOLL_CTL_ADD,fd,EPOLLIN|EPOLLET,21);watch(parent,EPOLL_CTL_ADD,child,EPOLLIN|EPOLLET,22);
    struct epoll_event e={.events=EPOLLIN};CHECK(epoll_ctl(child,EPOLL_CTL_ADD,parent,&e)==-1 && errno==ELOOP);
    write_value(fd,1);CHECK(epoll_wait(parent,&e,1,0)==1 && e.data.u64==22);
    CHECK(epoll_wait(child,&e,1,0)==1 && e.data.u64==21 && epoll_wait(parent,&e,1,0)==0);
    CHECK(close(fd)==0 && close(child)==0 && close(parent)==0);
}
int main(void) {
    counters();epoll_modes();lifetimes();contention();signals_and_copies();nested();
    // Process exit must cancel a blocked sibling and release its retained epoll.
    int ep=epoll_create1(0);CHECK(ep>=0);(void)launch(ep,9);
    puts("POLL_NATIVE_OK counter=1 semaphore=1 epoll=1 vectors=1 nested=1 lifetime=1 race=1 signals=1 timeout=1 pending_exit=1");
    return 0;
}
