#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/android/binder.h>
#include <pthread.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <ucontext.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr,"BINDER_CHECK_FAILED pid=%d line=%d errno=%d\n",getpid(),__LINE__,errno); _exit(91); } } while (0)
#define POOL_SIZE (128*1024)
static int control_fd, request_fd;
static _Atomic int phase;
static volatile sig_atomic_t faults, signals_seen;
static int flush_seen;
static void *pool;
struct payload { uint32_t pid, serial; uint64_t value; };
struct event { uint32_t command; struct binder_transaction_data transaction; uint64_t cookie; };

static void handler(int signal, siginfo_t *info, void *context) {
    if (signal==SIGUSR1) { ++signals_seen; return; }
    if (signal!=SIGSEGV || info->si_code!=SEGV_ACCERR || info->si_addr!=pool) _exit(92);
    ++faults; ((ucontext_t *)context)->uc_mcontext.pc+=4;
}
static void command(int fd, uint32_t code, const void *body, size_t length) {
    unsigned char bytes[68]; CHECK(length<=64);
    memcpy(bytes,&code,4); if (length) memcpy(bytes+4,body,length);
    struct binder_write_read io={.write_size=length+4,.write_buffer=(uintptr_t)bytes};
    CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==0 && io.write_consumed==io.write_size);
}
static void transmit(int fd, uint32_t operation, uint32_t code, uint32_t flags, const void *bytes, size_t size) {
    struct binder_transaction_data data={.code=code,.flags=flags,.sender_pid=0x7788,.sender_euid=0,
        .data_size=size,.data.ptr.buffer=(uintptr_t)bytes};
    command(fd,operation,&data,sizeof(data));
}
static struct event receive_from(int fd, int fallback) {
    for (;;) {
        unsigned char bytes[256];
        struct binder_write_read io={.read_size=sizeof(bytes),.read_buffer=(uintptr_t)bytes};
        CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==0 && io.read_consumed>=4 && io.read_consumed<=sizeof(bytes));
        struct event result={0}; size_t at=0;
        while (at<io.read_consumed) {
            uint32_t code; CHECK(io.read_consumed-at>=4); memcpy(&code,bytes+at,4); at+=4;
            size_t size=_IOC_SIZE(code); CHECK(size<=io.read_consumed-at);
            if (code==BR_TRANSACTION || code==BR_REPLY) { CHECK(!result.command); memcpy(&result.transaction,bytes+at,sizeof(result.transaction)); result.command=code; }
            else if (code==BR_DEAD_BINDER || code==BR_CLEAR_DEATH_NOTIFICATION_DONE) { CHECK(!result.command); memcpy(&result.cookie,bytes+at,8); result.command=code; }
            else if (code==BR_DEAD_REPLY || code==BR_FAILED_REPLY) { CHECK(!result.command); result.command=code; }
            else CHECK(code==BR_NOOP || code==BR_TRANSACTION_COMPLETE);
            at+=size;
        }
        if (result.command) return result;
        if(io.read_consumed==4 && fd!=fallback) {fd=fallback;flush_seen=1;}
    }
}
static struct event receive(int fd) {return receive_from(fd,fd);}
static void release_buffer(int fd, const struct event *event) {
    binder_uintptr_t address=event->transaction.data.ptr.buffer;
    CHECK(address>=(uintptr_t)pool && address+event->transaction.data_size<=(uintptr_t)pool+POOL_SIZE);
    command(fd,BC_FREE_BUFFER,&address,sizeof(address));
}
static void *waiting_reply(void *unused) {
    (void)unused;
    struct payload request={(uint32_t)getpid(),1,0x1122334455667788ULL};
    transmit(request_fd,BC_TRANSACTION,10,0,&request,sizeof(request));
    atomic_store(&phase,1);
    struct event reply=receive_from(request_fd,control_fd);
    CHECK(flush_seen==1);
    CHECK(reply.command==BR_REPLY && reply.transaction.sender_pid==0 && reply.transaction.sender_euid==(uint32_t)getuid());
    CHECK(reply.transaction.data_size==sizeof(request));
    struct payload answer; memcpy(&answer,(void *)(uintptr_t)reply.transaction.data.ptr.buffer,sizeof(answer));
    CHECK(answer.pid==request.pid && answer.serial==1 && answer.value==(request.value^UINT64_MAX));
    release_buffer(control_fd,&reply);
    CHECK(ioctl(control_fd,BINDER_THREAD_EXIT,NULL)==0);
    return NULL;
}
static void *interrupted_read(void *unused) {
    (void)unused;
    unsigned char bytes[256];
    struct binder_write_read io={.read_size=sizeof(bytes),.read_buffer=(uintptr_t)bytes};
    atomic_store(&phase,1);
    CHECK(ioctl(control_fd,BINDER_WRITE_READ,&io)==-1 && errno==EINTR);
    CHECK(io.read_consumed==0 && io.write_consumed==0);
    CHECK(ioctl(control_fd,BINDER_THREAD_EXIT,NULL)==0);
    return NULL;
}
static void client(int fd) {
    control_fd=dup(fd); CHECK(control_fd>=0); request_fd=fd;
    uint32_t handle=0; command(fd,BC_INCREFS,&handle,4); command(fd,BC_ACQUIRE,&handle,4);
    struct binder_handle_cookie death={.handle=0,.cookie=0x123456789abcdef0ULL};
    command(fd,BC_REQUEST_DEATH_NOTIFICATION,&death,sizeof(death));
    CHECK(fcntl(control_fd,F_SETFL,O_NONBLOCK)==0);
    unsigned char read_bytes[256];
    struct binder_write_read read_io={.read_size=sizeof(read_bytes),.read_buffer=(uintptr_t)read_bytes};
    CHECK(ioctl(control_fd,BINDER_WRITE_READ,&read_io)==-1 && errno==EAGAIN && read_io.read_consumed==0);
    CHECK(fcntl(control_fd,F_SETFL,0)==0);
    pthread_t worker; CHECK(pthread_create(&worker,NULL,waiting_reply,NULL)==0);
    while (!atomic_load(&phase)) usleep(1000);
    usleep(50000);
    CHECK(close(fd)==0); int reused=open("/dev/null",O_RDONLY); CHECK(reused==fd);
    CHECK(pthread_join(worker,NULL)==0 && close(reused)==0);
    atomic_store(&phase,0);
    CHECK(pthread_create(&worker,NULL,interrupted_read,NULL)==0);
    while (!atomic_load(&phase)) usleep(1000);
    usleep(50000); CHECK(pthread_kill(worker,SIGUSR1)==0 && pthread_join(worker,NULL)==0 && signals_seen==1);
    for (uint32_t i=1;i<=3;++i) {
        struct payload request={(uint32_t)getpid(),i,0xabc00000ULL+i};
        transmit(control_fd,BC_TRANSACTION,20,TF_ONE_WAY,&request,sizeof(request));
    }
    // No untrusted kernel pointer or unsupported object array is accepted.
    transmit(control_fd,BC_TRANSACTION,30,0,(void *)(uintptr_t)0x40080000,16);
    CHECK(receive(control_fd).command==BR_FAILED_REPLY);
    struct binder_transaction_data invalid={.offsets_size=8};
    command(control_fd,BC_TRANSACTION,&invalid,sizeof(invalid)); CHECK(receive(control_fd).command==BR_FAILED_REPLY);
    unsigned char large[8201];
    for (size_t i=0;i<sizeof(large);++i) large[i]=(unsigned char)(i^0x5a);
    transmit(control_fd,BC_TRANSACTION,40,0,large,sizeof(large));
    struct event reply=receive(control_fd); CHECK(reply.command==BR_REPLY && reply.transaction.data_size==sizeof(large));
    const unsigned char *answer=(void *)(uintptr_t)reply.transaction.data.ptr.buffer;
    for (size_t i=0;i<sizeof(large);++i) CHECK(answer[i]==(unsigned char)(large[i]^0xff));
    release_buffer(control_fd,&reply);
    struct payload invalid_destination={(uint32_t)getpid(),50,0x1234};
    transmit(control_fd,BC_TRANSACTION,50,0,&invalid_destination,sizeof(invalid_destination));
    CHECK(receive(control_fd).command==BR_FAILED_REPLY);
    struct payload finish={(uint32_t)getpid(),99,0x5555};
    transmit(control_fd,BC_TRANSACTION,99,0,&finish,sizeof(finish));
    reply=receive(control_fd); CHECK(reply.command==BR_REPLY); release_buffer(control_fd,&reply);
    // The service exits with this synchronous request still outstanding.
    transmit(control_fd,BC_TRANSACTION,100,0,&finish,sizeof(finish));
    CHECK(receive(control_fd).command==BR_DEAD_REPLY);
    struct event notified=receive(control_fd); CHECK(notified.command==BR_DEAD_BINDER && notified.cookie==death.cookie);
    command(control_fd,BC_CLEAR_DEATH_NOTIFICATION,&death,sizeof(death));
    command(control_fd,BC_DEAD_BINDER_DONE,&death.cookie,8);
    CHECK(receive(control_fd).command==BR_CLEAR_DEATH_NOTIFICATION_DONE);
    transmit(control_fd,BC_TRANSACTION,101,0,&finish,sizeof(finish)); CHECK(receive(control_fd).command==BR_DEAD_REPLY);
    command(control_fd,BC_RELEASE,&handle,4); command(control_fd,BC_DECREFS,&handle,4);
    CHECK(munmap(pool,POOL_SIZE)==0 && close(control_fd)==0);
    puts("BINDER_CLIENT_OK identity=1 reply=1 oneway=3 death=1 interrupted=1 fd_reuse=1 readonly=1");
}
static void server(int fd) {
    int hardware=open("/dev/hwbinder",O_RDWR|O_CLOEXEC);
    int vendor=open("/dev/vndbinder",O_RDWR|O_CLOEXEC);
    CHECK(hardware>=0 && vendor>=0);
    // Context managers are per device, not aliases of the framework service.
    CHECK(ioctl(hardware,BINDER_SET_CONTEXT_MGR,NULL)==0 && ioctl(vendor,BINDER_SET_CONTEXT_MGR,NULL)==0);
    int collision=open("/dev/hwbinder",O_RDWR|O_CLOEXEC);CHECK(collision>=0);
    CHECK(ioctl(collision,BINDER_SET_CONTEXT_MGR,NULL)==-1 && errno==EBUSY);
    CHECK(close(collision)==0 && close(hardware)==0 && close(vendor)==0);
    command(fd,BC_ENTER_LOOPER,NULL,0);
    uint32_t oneway=0, client_pid=0;
    for (;;) {
        struct event request=receive(fd); CHECK(request.command==BR_TRANSACTION);
        CHECK(request.transaction.sender_euid==(uint32_t)getuid());
        uint32_t code=request.transaction.code;
        if (code==40) {
            CHECK(oneway==3 && request.transaction.sender_pid==(int32_t)client_pid && request.transaction.data_size==8201);
            unsigned char response[8201]; const unsigned char *data=(void *)(uintptr_t)request.transaction.data.ptr.buffer;
            for (size_t i=0;i<sizeof(response);++i) { CHECK(data[i]==(unsigned char)(i^0x5a)); response[i]=data[i]^0xff; }
            release_buffer(fd,&request); transmit(fd,BC_REPLY,40,0,response,sizeof(response));
            unsigned char completed[256];
            struct binder_write_read io={.read_size=sizeof(completed),.read_buffer=(uintptr_t)completed};
            CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==0 && io.read_consumed==8);
            uint32_t marker; memcpy(&marker,completed+4,4); CHECK(marker==BR_TRANSACTION_COMPLETE);
            io=(struct binder_write_read){.read_size=256,.read_buffer=0x40080000};
            CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==-1 && errno==EFAULT && io.read_consumed==0);
            continue;
        }
        CHECK(request.transaction.data_size==sizeof(struct payload));
        struct payload value; memcpy(&value,(void *)(uintptr_t)request.transaction.data.ptr.buffer,sizeof(value));
        CHECK(value.pid!=(uint32_t)getpid());
        if (code==20) {
            CHECK(request.transaction.sender_pid==0 && value.serial==++oneway && value.value==0xabc00000ULL+oneway);
            release_buffer(fd,&request); continue;
        }
        CHECK(request.transaction.sender_pid==(int32_t)value.pid);
        if (code==10) { client_pid=value.pid; usleep(300000); CHECK(value.serial==1 && value.value==0x1122334455667788ULL); }
        else CHECK((code==99 || code==100) && oneway==3 && value.pid==client_pid);
        if (code==100) { release_buffer(fd,&request); break; }
        release_buffer(fd,&request); value.value^=UINT64_MAX; transmit(fd,BC_REPLY,code,0,&value,sizeof(value));
    }
    // Close the service first: unmapping an active receive pool intentionally
    // fails its outstanding work before a later close could report peer death.
    CHECK(close(fd)==0 && munmap(pool,POOL_SIZE)==0);
    puts("BINDER_SERVER_OK identity=1 reply=1 oneway=3 receive_pool=1 readonly=1 domains=3");
}
int main(void) {
    int fd=open("/dev/binder",O_RDWR|O_CLOEXEC); CHECK(fd>=0);
    struct binder_version version={0}; CHECK(ioctl(fd,BINDER_VERSION,&version)==0 && version.protocol_version==8);
    CHECK(ioctl(fd,BINDER_VERSION,(void *)(uintptr_t)0x40080000)==-1 && errno==EFAULT);
    CHECK(ioctl(fd,0,NULL)==-1 && errno==EINVAL);
    struct stat status; CHECK(fstat(fd,&status)==0 && S_ISCHR(status.st_mode));
    CHECK(ftruncate(fd,4096)==-1 && errno==EINVAL);
    CHECK(ioctl(fd,BINDER_ENABLE_ONEWAY_SPAM_DETECTION,&version)==-1 && errno==EINVAL);
    uint32_t maximum=0; CHECK(ioctl(fd,BINDER_SET_MAX_THREADS,&maximum)==0);
    pool=mmap(NULL,POOL_SIZE,PROT_READ,MAP_PRIVATE|MAP_NORESERVE,fd,0); CHECK(pool!=MAP_FAILED);
    CHECK(mmap(NULL,4096,PROT_READ,MAP_PRIVATE,fd,0)==MAP_FAILED && errno==EBUSY);
    CHECK(mprotect(pool,4096,PROT_READ|PROT_WRITE)==-1 && errno==EACCES);
    CHECK(mprotect(pool,4096,PROT_READ|PROT_EXEC)==-1 && errno==EACCES);
    struct sigaction action={.sa_sigaction=handler,.sa_flags=SA_SIGINFO}; sigemptyset(&action.sa_mask);
    CHECK(sigaction(SIGSEGV,&action,NULL)==0 && sigaction(SIGUSR1,&action,NULL)==0);
    __asm__ volatile("strb %w1, [%0]" :: "r"(pool),"r"(255) : "memory");
    CHECK(faults==1 && *(const unsigned char *)pool==0);
    struct binder_write_read invalid={.write_size=4,.write_consumed=5};
    CHECK(ioctl(fd,BINDER_WRITE_READ,&invalid)==-1 && errno==EINVAL);
    int manager=ioctl(fd,BINDER_SET_CONTEXT_MGR,NULL);
    if (manager==0) server(fd); else { CHECK(errno==EBUSY); client(fd); }
    return 0;
}
