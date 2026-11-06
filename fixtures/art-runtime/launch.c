#define _GNU_SOURCE
#include <android/binder_ibinder.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/android/binder.h>
#include <pthread.h>
#include <poll.h>
#include <stdatomic.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/epoll.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/timerfd.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr,"LAUNCH_FAIL line=%d errno=%d uid=%u\n",__LINE__,errno,getuid());exit(61); } } while (0)
static volatile sig_atomic_t signals_seen;
static pthread_t leader;
static atomic_bool lookup_done;
static void *lookup_watchdog(void *unused) {
    (void)unused;for(int i=0;i<500;++i) {if(atomic_load(&lookup_done))return NULL;usleep(10000);}
    fputs("SERVICE_MANAGER_CLIENT_TIMEOUT\n",stderr);_exit(63);
}
static void handler(int signal,siginfo_t *info,void *context) {
    (void)context;
    if(signal!=SIGUSR1 || info->si_uid!=getuid() || info->si_pid!=getpid()) _exit(62);
    ++signals_seen;
}
static void *interrupt_read(void *unused) {
    (void)unused;usleep(20000);CHECK(pthread_kill(leader,SIGUSR1)==0);return NULL;
}
static void timers(void) {
    int fd=timerfd_create(CLOCK_MONOTONIC,TFD_NONBLOCK|TFD_CLOEXEC);CHECK(fd>=0);
    CHECK(fcntl(fd,F_GETFD)==FD_CLOEXEC && (fcntl(fd,F_GETFL)&O_NONBLOCK));
    char link[64];char path[64];snprintf(path,sizeof(path),"/proc/self/fd/%d",fd);
    ssize_t n=readlink(path,link,sizeof(link));CHECK(n==(ssize_t)sizeof("anon_inode:[timerfd]")-1 && memcmp(link,"anon_inode:[timerfd]",(size_t)n)==0);
    struct itimerspec setting={.it_interval={0,5000000},.it_value={0,1000000}},old;
    CHECK(timerfd_settime(fd,0,&setting,NULL)==0);usleep(35000);
    uint64_t ticks=0;CHECK(read(fd,&ticks,8)==8 && ticks>=3);
    CHECK(timerfd_gettime(fd,&old)==0 && old.it_interval.tv_nsec==5000000 && old.it_value.tv_sec==0 && old.it_value.tv_nsec<=5000000);
    struct itimerspec bad=setting;bad.it_value.tv_nsec=1000000000;
    CHECK(timerfd_settime(fd,0,&bad,NULL)==-1 && errno==EINVAL);
    CHECK(timerfd_settime(fd,2,&setting,NULL)==-1 && errno==EINVAL);
    int ep=epoll_create1(EPOLL_CLOEXEC);CHECK(ep>=0);
    struct epoll_event event={.events=EPOLLIN|EPOLLET|EPOLLONESHOT,.data.u64=0x71};
    CHECK(epoll_ctl(ep,EPOLL_CTL_ADD,fd,&event)==0);
    setting=(struct itimerspec){.it_value={0,20000000}};
    CHECK(timerfd_settime(fd,0,&setting,&old)==0 && old.it_interval.tv_nsec==5000000);
    CHECK(epoll_wait(ep,&event,1,1000)==1 && event.data.u64==0x71 && (event.events&EPOLLIN));
    int duplicate=dup(fd);CHECK(duplicate>=0 && close(fd)==0);
    int reused=open("/dev/null",O_RDONLY);CHECK(reused==fd);
    CHECK(read(duplicate,&ticks,8)==8 && ticks==1);
    CHECK(read(duplicate,&ticks,8)==-1 && errno==EAGAIN);
    CHECK(timerfd_gettime(duplicate,&old)==0 && old.it_value.tv_sec==0 && old.it_value.tv_nsec==0);
    CHECK(close(reused)==0 && close(duplicate)==0 && close(ep)==0);
    for(int clock_index=0;clock_index<2;++clock_index) {
        clockid_t clock=clock_index==0?CLOCK_REALTIME:CLOCK_BOOTTIME;
        fd=timerfd_create(clock,0);CHECK(fd>=0);
        CHECK(clock_gettime(clock,&setting.it_value)==0);setting.it_interval=(struct timespec){0};
        CHECK(timerfd_settime(fd,TFD_TIMER_ABSTIME,&setting,NULL)==0);
        CHECK(read(fd,&ticks,8)==8 && ticks==1 && close(fd)==0);
    }
    fd=timerfd_create(CLOCK_MONOTONIC,0);CHECK(fd>=0);leader=pthread_self();
    pthread_t worker;CHECK(pthread_create(&worker,NULL,interrupt_read,NULL)==0);
    CHECK(read(fd,&ticks,8)==-1 && errno==EINTR);
    CHECK(pthread_join(worker,NULL)==0 && signals_seen==2 && close(fd)==0);
    fd=timerfd_create(CLOCK_MONOTONIC,TFD_NONBLOCK);CHECK(fd>=0);
    setting=(struct itimerspec){.it_value={0,20000000}};CHECK(timerfd_settime(fd,0,&setting,NULL)==0);
    int metadata=open("/dev/null",O_PATH);CHECK(metadata>=0);
    struct pollfd polled[4]={{.fd=fd,.events=POLLIN},{.fd=-1,.events=POLLIN},{.fd=9999,.events=POLLIN},{.fd=metadata,.events=POLLIN}};
    struct timespec timeout={0};CHECK(ppoll(polled,4,&timeout,NULL)==2 && polled[1].revents==0 && polled[2].revents==POLLNVAL && polled[3].revents==POLLNVAL);
    CHECK(close(metadata)==0);
    timeout=(struct timespec){1,0};CHECK(ppoll(polled,2,&timeout,NULL)==1 && polled[0].revents==POLLIN);
    CHECK(read(fd,&ticks,8)==8 && ticks==1 && close(fd)==0);
    timeout=(struct timespec){0,1000000};CHECK(ppoll(NULL,0,&timeout,NULL)==0);
    puts("LAUNCH_PPOLL_OK invalid=1 ignored=1 timer_wake=1 timeout=1");
    puts("LAUNCH_TIMERS_OK periodic=1 absolute=1 poll=1 lifetime=1 interrupted=1");
}
static void command(int fd,uint32_t op,const void *payload,size_t size) {
    unsigned char bytes[80];CHECK(size+4<=sizeof(bytes));memcpy(bytes,&op,4);
    if(size) memcpy(bytes+4,payload,size);
    struct binder_write_read io={.write_size=size+4,.write_buffer=(uintptr_t)bytes};
    CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==0 && io.write_consumed==size+4);
}
static void send_value(int fd,uint32_t op,uint32_t value) {
    struct binder_transaction_data tx={.code=1,.sender_pid=-1,.sender_euid=0,.data_size=4,.data.ptr.buffer=(uintptr_t)&value};
    command(fd,op,&tx,sizeof(tx));
}
static uint32_t receive(int fd,struct binder_transaction_data *tx) {
    for(;;) {
        unsigned char bytes[256];struct binder_write_read io={.read_size=sizeof(bytes),.read_buffer=(uintptr_t)bytes};
        CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==0);
        size_t at=0;
        while(at<io.read_consumed) {
            uint32_t code;memcpy(&code,bytes+at,4);at+=4;
            if(code==BR_NOOP || code==BR_TRANSACTION_COMPLETE) continue;
            if(code==BR_TRANSACTION || code==BR_REPLY) {CHECK(at+sizeof(*tx)<=io.read_consumed);memcpy(tx,bytes+at,sizeof(*tx));return code;}
            CHECK(code==BR_DEAD_REPLY || code==BR_FAILED_REPLY);return code;
        }
    }
}
static void identity_binder(int system) {
    int fd=open("/dev/binder",O_RDWR|O_CLOEXEC);CHECK(fd>=0);
    void *pool=mmap(NULL,65536,PROT_READ,MAP_PRIVATE,fd,0);CHECK(pool!=MAP_FAILED);
    command(fd,BC_ENTER_LOOPER,NULL,0);
    if(system) CHECK(ioctl(fd,BINDER_SET_CONTEXT_MGR,0)==0);
    else CHECK(ioctl(fd,BINDER_SET_CONTEXT_MGR,0)==-1 && errno==EPERM);
    struct binder_transaction_data tx={0};uint32_t code;
    if(system) {
        CHECK(receive(fd,&tx)==BR_TRANSACTION && tx.sender_euid==10000 && tx.sender_pid!=getpid());
        CHECK(tx.data_size==4 && *(const uint32_t *)(uintptr_t)tx.data.ptr.buffer==0x1234);
        binder_uintptr_t buffer=tx.data.ptr.buffer;command(fd,BC_FREE_BUFFER,&buffer,sizeof(buffer));
        send_value(fd,BC_REPLY,0x5678);
    } else {
        unsigned tries=0;
        do { send_value(fd,BC_TRANSACTION,0x1234);code=receive(fd,&tx);if(code==BR_REPLY)break;CHECK(++tries<1000);usleep(1000); } while(1);
        // Linux leaves reply sender_pid zero, but authenticates sender_euid.
        CHECK(tx.sender_euid==1000 && tx.data_size==4 && *(const uint32_t *)(uintptr_t)tx.data.ptr.buffer==0x5678);
        binder_uintptr_t buffer=tx.data.ptr.buffer;command(fd,BC_FREE_BUFFER,&buffer,sizeof(buffer));
    }
    CHECK(munmap(pool,65536)==0 && close(fd)==0);
    printf("LAUNCH_IDENTITY_OK role=%s uid=%u gid=%u auxv=1 signals=1 binder=1 authority=1\n",system?"system":"app",getuid(),getgid());
}
static void lookup_manager(void) {
    pthread_t watchdog;CHECK(pthread_create(&watchdog,NULL,lookup_watchdog,NULL)==0);
    void *library=dlopen("libbinder_ndk.so",RTLD_NOW);CHECK(library!=NULL);
    AIBinder *(*check)(const char *)=dlsym(library,"AServiceManager_checkService");CHECK(check!=NULL);
    AIBinder *manager=NULL;
    for(int attempt=0;attempt<100 && manager==NULL;++attempt) {manager=check("manager");if(!manager)usleep(10000);}
    CHECK(manager!=NULL && AIBinder_ping(manager)==STATUS_OK);
    AIBinder_decStrong(manager);
    atomic_store(&lookup_done,1);CHECK(pthread_join(watchdog,NULL)==0);
    puts("SERVICE_MANAGER_CLIENT_OK upstream=1 lookup=1 ping=1 uid=10000");
}
int main(int argc,char **argv) {
    CHECK(argc==2);int system=strcmp(argv[1],"system")==0;uid_t expected=system?1000:10000;
    CHECK(getuid()==expected && geteuid()==expected && getgid()==expected && getegid()==expected);
    CHECK(getauxval(AT_UID)==expected && getauxval(AT_EUID)==expected && getauxval(AT_GID)==expected && getauxval(AT_EGID)==expected);
    uid_t real,effective,saved;CHECK(getresuid(&real,&effective,&saved)==0 && real==expected && effective==expected && saved==expected);
    CHECK(getresgid(&real,&effective,&saved)==0 && real==expected && effective==expected && saved==expected && getgroups(0,NULL)==0);
    if(strcmp(argv[1],"lookup")==0) {lookup_manager();return 0;}
    CHECK(system || strcmp(argv[1],"app")==0);
    struct sigaction action={.sa_sigaction=handler,.sa_flags=SA_SIGINFO};CHECK(sigaction(SIGUSR1,&action,NULL)==0 && raise(SIGUSR1)==0 && signals_seen==1);
    timers();identity_binder(system);return 0;
}
