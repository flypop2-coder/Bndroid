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
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <sys/syscall.h>
#include <linux/memfd.h>
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
static uint32_t object_handles[2];
static unsigned ref_incs, ref_acquires, ref_releases, ref_decs;
#define CLIENT_PTR 0x123400
#define SERVER_PTR 0x567800
#define COOKIE(p) ((p)+0x55)
static void *pool;
static void *fd_fault_address;
static volatile sig_atomic_t fd_faults;
static int binder_poll=-1, binder_outer=-1, polled_binder=-1;
static unsigned binder_poll_wakes;
static int survivor_fd=-1;
static const char *survivor_map;
struct payload { uint32_t pid, serial; uint64_t value; };
struct event { uint32_t command; struct binder_transaction_data transaction; uint64_t cookie; };

static void handler(int signal, siginfo_t *info, void *context) {
    if(signal==SIGBUS && info->si_code==BUS_ADRERR && info->si_addr==fd_fault_address) {
        ++fd_faults;((ucontext_t *)context)->uc_mcontext.pc+=4;return;
    }
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
        int status=ioctl(fd,BINDER_WRITE_READ,&io);
        if(status==-1 && errno==EAGAIN && binder_poll>=0 && fd==polled_binder) {
            struct epoll_event event;
            CHECK(epoll_wait(binder_outer,&event,1,5000)==1 && event.events&EPOLLIN && event.data.u64==0xb2);
            CHECK(epoll_wait(binder_poll,&event,1,0)==1 && event.events&EPOLLIN && event.data.u64==0xb1);
            ++binder_poll_wakes;continue;
        }
        CHECK(status==0 && io.read_consumed>=4 && io.read_consumed<=sizeof(bytes));
        struct event result={0}; size_t at=0;
        while (at<io.read_consumed) {
            uint32_t code; CHECK(io.read_consumed-at>=4); memcpy(&code,bytes+at,4); at+=4;
            size_t size=_IOC_SIZE(code); CHECK(size<=io.read_consumed-at);
            if (code==BR_TRANSACTION || code==BR_REPLY) { CHECK(!result.command); memcpy(&result.transaction,bytes+at,sizeof(result.transaction)); result.command=code; }
            else if (code==BR_DEAD_BINDER || code==BR_CLEAR_DEATH_NOTIFICATION_DONE) { CHECK(!result.command); memcpy(&result.cookie,bytes+at,8); result.command=code; }
            else if (code==BR_DEAD_REPLY || code==BR_FAILED_REPLY) { CHECK(!result.command); result.command=code; }
            else if (code==BR_INCREFS || code==BR_ACQUIRE || code==BR_RELEASE || code==BR_DECREFS) {
                struct binder_ptr_cookie ref; memcpy(&ref,bytes+at,sizeof(ref));
                CHECK((ref.ptr==CLIENT_PTR || ref.ptr==CLIENT_PTR+0x100 || ref.ptr==SERVER_PTR || ref.ptr==SERVER_PTR+0x100 || ref.ptr==SERVER_PTR+0x200) && ref.cookie==COOKIE(ref.ptr));
                if(code==BR_INCREFS) {++ref_incs;command(fd,BC_INCREFS_DONE,&ref,sizeof(ref));}
                else if(code==BR_ACQUIRE) {++ref_acquires;command(fd,BC_ACQUIRE_DONE,&ref,sizeof(ref));}
                else if(code==BR_RELEASE) ++ref_releases;
                else ++ref_decs;
            }
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
static struct flat_binder_object local_object(uintptr_t ptr, int strong) {
    return (struct flat_binder_object){.hdr.type=strong?BINDER_TYPE_BINDER:BINDER_TYPE_WEAK_BINDER,
        .flags=FLAT_BINDER_FLAG_ACCEPTS_FDS,.binder=ptr,.cookie=COOKIE(ptr)};
}
static struct flat_binder_object remote_object(uint32_t handle, int strong) {
    return (struct flat_binder_object){.hdr.type=strong?BINDER_TYPE_HANDLE:BINDER_TYPE_WEAK_HANDLE,
        .flags=FLAT_BINDER_FLAG_ACCEPTS_FDS,.handle=handle};
}
static void transmit_objects(int fd,uint32_t operation,uint32_t handle,uint32_t code,uint32_t flags,
                             struct flat_binder_object *objects,size_t count) {
    binder_size_t offsets[4];CHECK(count<=4);
    for(size_t i=0;i<count;++i) offsets[i]=i*sizeof(*objects);
    struct binder_transaction_data data={.target.handle=handle,.code=code,.flags=flags,
        .data_size=count*sizeof(*objects),.offsets_size=count*sizeof(*offsets),
        .data.ptr.buffer=(uintptr_t)objects,.data.ptr.offsets=(uintptr_t)offsets};
    // Only target.handle's low 32 bits are meaningful on a write. Deliberately
    // poison the LP64 union padding instead of depending on compiler choices.
    data.target.ptr=0xaabbccdd00000000ULL|handle;
    command(fd,operation,&data,sizeof(data));
}
static struct flat_binder_object get_object(const struct event *event,size_t index,size_t count) {
    CHECK(event->transaction.data_size==count*sizeof(struct flat_binder_object) && event->transaction.offsets_size==count*8);
    uintptr_t base=event->transaction.data.ptr.buffer,offsets=event->transaction.data.ptr.offsets;
    CHECK(base>=(uintptr_t)pool && offsets==base+count*sizeof(struct flat_binder_object) && offsets+count*8<=(uintptr_t)pool+POOL_SIZE);
    binder_size_t offset;memcpy(&offset,(void *)(offsets+index*8),8);CHECK(offset==index*sizeof(struct flat_binder_object));
    struct flat_binder_object object;memcpy(&object,(void *)(base+offset),sizeof(object));return object;
}
static uint32_t retain_object(int fd,struct flat_binder_object object,int strong) {
    CHECK(object.hdr.type==(strong?BINDER_TYPE_HANDLE:BINDER_TYPE_WEAK_HANDLE) && object.handle>0 && object.cookie==0);
    command(fd,BC_INCREFS,&object.handle,4);if(strong) command(fd,BC_ACQUIRE,&object.handle,4);return object.handle;
}
static void object_client(int fd) {
    struct flat_binder_object objects[4]={local_object(CLIENT_PTR,1),local_object(CLIENT_PTR+0x100,0)};
    transmit_objects(fd,BC_TRANSACTION,0,60,0,objects,2);
    struct event response=receive(fd);if(response.command!=BR_REPLY) fprintf(stderr,"BINDER_OBJECT_RESPONSE command=%x\n",response.command);CHECK(response.command==BR_REPLY);
    for(size_t i=0;i<2;++i) object_handles[i]=retain_object(fd,get_object(&response,i,4),1);
    for(size_t i=0;i<2;++i) {struct flat_binder_object o=get_object(&response,i+2,4);
        CHECK(o.hdr.type==(i?BINDER_TYPE_WEAK_BINDER:BINDER_TYPE_BINDER) && o.binder==CLIENT_PTR+i*0x100 && o.cookie==COOKIE(o.binder));}
    release_buffer(fd,&response);
    // Partial translation must roll back the first export when the second
    // object is unsupported or overlaps the first object.
    struct flat_binder_object bad[2]={local_object(CLIENT_PTR+0x200,1),{.hdr.type=BINDER_TYPE_FD,.handle=fd}};
    transmit_objects(fd,BC_TRANSACTION,object_handles[0],61,0,bad,2);CHECK(receive(fd).command==BR_FAILED_REPLY);
    bad[1]=local_object(CLIENT_PTR+0x300,1);binder_size_t offsets[2]={0,4};
    struct binder_transaction_data invalid={.target.handle=object_handles[0],.data_size=sizeof(bad),.offsets_size=sizeof(offsets),
        .data.ptr.buffer=(uintptr_t)bad,.data.ptr.offsets=(uintptr_t)offsets};
    command(fd,BC_TRANSACTION,&invalid,sizeof(invalid));CHECK(receive(fd).command==BR_FAILED_REPLY);
    transmit_objects(fd,BC_TRANSACTION,0x7fffffff,61,0,NULL,0);CHECK(receive(fd).command==BR_FAILED_REPLY);
    for(size_t i=0;i<2;++i) {struct binder_handle_cookie d={.handle=object_handles[i],.cookie=0xcafe0000+i};
        command(fd,BC_REQUEST_DEATH_NOTIFICATION,&d,sizeof(d));}
    objects[0]=remote_object(object_handles[0],1);
    transmit_objects(fd,BC_TRANSACTION,object_handles[0],61,0,objects,1);
    response=receive(fd);CHECK(response.command==BR_REPLY);release_buffer(fd,&response);
    for(size_t i=0;i<3;++i) transmit_objects(fd,BC_TRANSACTION,object_handles[i==2?1:0],62,TF_ONE_WAY,NULL,0);
    transmit_objects(fd,BC_TRANSACTION,object_handles[1],63,0,NULL,0);
    struct event callback=receive(fd);CHECK(callback.command==BR_TRANSACTION && callback.transaction.code==64 && callback.transaction.target.ptr==CLIENT_PTR && callback.transaction.cookie==COOKIE(CLIENT_PTR));
    CHECK(callback.transaction.sender_pid>0 && callback.transaction.sender_pid!=getpid() && callback.transaction.sender_euid==(uint32_t)getuid());
    release_buffer(fd,&callback);
    // Re-enter the waiting service before returning the first callback. Both
    // sides must receive this on their original synchronous call threads.
    transmit_objects(fd,BC_TRANSACTION,object_handles[0],65,0,NULL,0);
    response=receive(fd);CHECK(response.command==BR_REPLY);release_buffer(fd,&response);
    transmit_objects(fd,BC_REPLY,0,64,0,NULL,0);
    response=receive(fd);CHECK(response.command==BR_REPLY);release_buffer(fd,&response);
}
static void object_server(int fd,const struct event *request) {
    uint32_t handles[2];handles[0]=retain_object(fd,get_object(request,0,2),1);handles[1]=retain_object(fd,get_object(request,1,2),0);
    release_buffer(fd,request);
    struct flat_binder_object objects[4]={local_object(SERVER_PTR,1),local_object(SERVER_PTR+0x100,1),remote_object(handles[0],1),remote_object(handles[1],0)};
    transmit_objects(fd,BC_REPLY,0,60,0,objects,4);
    struct event next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==61 && next.transaction.target.ptr==SERVER_PTR && next.transaction.cookie==COOKIE(SERVER_PTR));
    struct flat_binder_object local=get_object(&next,0,1);CHECK(local.hdr.type==BINDER_TYPE_BINDER && local.binder==SERVER_PTR && local.cookie==COOKIE(SERVER_PTR));
    release_buffer(fd,&next);transmit_objects(fd,BC_REPLY,0,61,0,NULL,0);
    struct event first=receive(fd);CHECK(first.command==BR_TRANSACTION && first.transaction.code==62 && first.transaction.target.ptr==SERVER_PTR && first.transaction.sender_pid==0);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==62 && next.transaction.target.ptr==SERVER_PTR+0x100 && next.transaction.sender_pid==0);
    release_buffer(fd,&first);release_buffer(fd,&next);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==62 && next.transaction.target.ptr==SERVER_PTR && next.transaction.sender_pid==0);release_buffer(fd,&next);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==63 && next.transaction.target.ptr==SERVER_PTR+0x100);release_buffer(fd,&next);
    transmit_objects(fd,BC_TRANSACTION,handles[0],64,0,NULL,0);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==65 && next.transaction.target.ptr==SERVER_PTR);
    release_buffer(fd,&next);transmit_objects(fd,BC_REPLY,0,65,0,NULL,0);
    next=receive(fd);CHECK(next.command==BR_REPLY);release_buffer(fd,&next);
    transmit_objects(fd,BC_REPLY,0,63,0,NULL,0);
    command(fd,BC_RELEASE,&handles[0],4);for(size_t i=0;i<2;++i) command(fd,BC_DECREFS,&handles[i],4);
    CHECK(ref_incs==2 && ref_acquires==2);
    puts("BINDER_OBJECT_SERVER_OK export=2 callback=1 nested=2 per_node_oneway=1 refs=1");
}
#include "binder_files.h"

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
    object_client(control_fd);
    files_client(control_fd);
    // The service exits with this synchronous request still outstanding.
    transmit(control_fd,BC_TRANSACTION,100,0,&finish,sizeof(finish));
    CHECK(receive(control_fd).command==BR_DEAD_REPLY);
    unsigned notified_mask=0;
    for(size_t i=0;i<3;++i) {
        struct event notified=receive(control_fd);CHECK(notified.command==BR_DEAD_BINDER);
        struct binder_handle_cookie d=death;unsigned bit=1;
        if(notified.cookie!=death.cookie) {CHECK(notified.cookie>=0xcafe0000 && notified.cookie<0xcafe0002);
            size_t index=notified.cookie-0xcafe0000;d.handle=object_handles[index];d.cookie=notified.cookie;bit=2U<<index;}
        CHECK(!(notified_mask&bit));notified_mask|=bit;
        command(control_fd,BC_CLEAR_DEATH_NOTIFICATION,&d,sizeof(d));command(control_fd,BC_DEAD_BINDER_DONE,&d.cookie,8);
        struct event clear=receive(control_fd);CHECK(clear.command==BR_CLEAR_DEATH_NOTIFICATION_DONE && clear.cookie==d.cookie);
    }
    CHECK(notified_mask==7 && ref_incs==2 && ref_acquires==1 && ref_releases==1 && ref_decs==2);
    for(size_t i=0;i<2;++i) {
        transmit_objects(control_fd,BC_TRANSACTION,object_handles[i],61,0,NULL,0);CHECK(receive(control_fd).command==BR_DEAD_REPLY);
        command(control_fd,BC_RELEASE,&object_handles[i],4);command(control_fd,BC_DECREFS,&object_handles[i],4);
    }
    puts("BINDER_OBJECT_CLIENT_OK export=2 handles=2 roundtrip=1 malformed=3 deaths=2 refs=1");
    transmit(control_fd,BC_TRANSACTION,101,0,&finish,sizeof(finish)); CHECK(receive(control_fd).command==BR_DEAD_REPLY);
    command(control_fd,BC_RELEASE,&handle,4); command(control_fd,BC_DECREFS,&handle,4);
    char survived[16];CHECK(pread(survivor_fd,survived,15,0)==15 && memcmp(survived,"binder survives",15)==0);
    CHECK(memcmp(survivor_map,"binder survives",15)==0 && munmap((void *)survivor_map,4096)==0 && close(survivor_fd)==0);
    CHECK(fd_faults==1 && binder_poll_wakes>0);
    CHECK(munmap(pool,POOL_SIZE)==0 && close(control_fd)==0);stop_binder_poll();
    puts("BINDER_FD_CLIENT_OK shared=1 offset=1 cloexec=1 truncate=1 regrow=1 seals=1 full=1 peer_exit=1 poll=1");
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
        if (code==60) {object_server(fd,&request);continue;}
        if (code==70) {files_server(fd,&request);continue;}
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
    CHECK(close(fd)==0 && munmap(pool,POOL_SIZE)==0);stop_binder_poll();
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
    CHECK(sigaction(SIGSEGV,&action,NULL)==0 && sigaction(SIGUSR1,&action,NULL)==0 && sigaction(SIGBUS,&action,NULL)==0);
    __asm__ volatile("strb %w1, [%0]" :: "r"(pool),"r"(255) : "memory");
    CHECK(faults==1 && *(const unsigned char *)pool==0);
    struct binder_write_read invalid={.write_size=4,.write_consumed=5};
    CHECK(ioctl(fd,BINDER_WRITE_READ,&invalid)==-1 && errno==EINVAL);
    struct flat_binder_object manager_node={.flags=FLAT_BINDER_FLAG_ACCEPTS_FDS};
    int manager=ioctl(fd,BINDER_SET_CONTEXT_MGR_EXT,&manager_node);
    if (manager==0) server(fd); else { CHECK(errno==EBUSY); client(fd); }
    return 0;
}
