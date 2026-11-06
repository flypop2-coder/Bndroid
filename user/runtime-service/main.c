#define _GNU_SOURCE
#include "protocol.h"
#include <errno.h>
#include <inttypes.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <sys/random.h>
#include <sys/utsname.h>
#include <time.h>
#include <unistd.h>

struct runtime { struct platform_api api; uint64_t instance; bool stopping; };
static void *create(void *state) { return state; }
static void destroy(void *state) { (void)state; }
static binder_status_t transact(AIBinder *binder,transaction_code_t code,const AParcel *in,AParcel *out) {
    struct runtime *state=AIBinder_getUserData(binder);
    int32_t version;
    if(AParcel_readInt32(in,&version)!=STATUS_OK || version!=BNDROID_RUNTIME_VERSION || AParcel_getDataPosition(in)!=AParcel_getDataSize(in))
        return platform_status(out,EX_ILLEGAL_ARGUMENT);
    const char *sid=state->api.calling_sid();
    if(!sid || state->api.check_access(sid,BNDROID_RUNTIME_CONTEXT,"service_manager","find",NULL)!=0)
        return platform_status(out,EX_SECURITY);
    if(code==BNDROID_RUNTIME_STOP) {
        if(state->api.check_access(sid,BNDROID_RUNTIME_CONTEXT,"bndroid_service","stop",NULL)!=0)
            return platform_status(out,EX_SECURITY);
        binder_status_t result=platform_status(out,EX_NONE);
        if(result==STATUS_OK)state->stopping=true;
        return result;
    }
    if(code!=BNDROID_RUNTIME_SNAPSHOT)return STATUS_UNKNOWN_TRANSACTION;
    struct utsname system;struct timespec now;
    if(uname(&system)!=0 || clock_gettime(CLOCK_BOOTTIME,&now)!=0)return platform_status(out,EX_ILLEGAL_STATE);
    binder_status_t result;
#define WRITE(call) do {result=(call);if(result!=STATUS_OK)return result;}while(0)
    WRITE(platform_status(out,EX_NONE));
    WRITE(AParcel_writeInt32(out,BNDROID_RUNTIME_VERSION));
    WRITE(AParcel_writeUint64(out,state->instance));
    WRITE(AParcel_writeInt64(out,(int64_t)now.tv_sec*1000000000+now.tv_nsec));
    WRITE(AParcel_writeInt32(out,getpid()));WRITE(AParcel_writeInt32(out,getuid()));
    WRITE(AParcel_writeInt32(out,AIBinder_getCallingPid()));WRITE(AParcel_writeInt32(out,AIBinder_getCallingUid()));
    WRITE(AParcel_writeString(out,system.nodename,(int32_t)strlen(system.nodename)));
    WRITE(AParcel_writeString(out,system.release,(int32_t)strlen(system.release)));
    WRITE(AParcel_writeString(out,system.machine,(int32_t)strlen(system.machine)));
#undef WRITE
    return STATUS_OK;
}
int main(void) {
    setvbuf(stdout,NULL,_IOLBF,0);
    struct runtime state={0};
    if(!platform_api_open(&state.api) || !state.api.pool_max(0))return 71;
    ssize_t random;do {random=getrandom(&state.instance,sizeof(state.instance),0);}while(random<0 && errno==EINTR);
    if(random!=(ssize_t)sizeof(state.instance) || state.instance==0)return 72;
    AIBinder_Class *type=AIBinder_Class_define(BNDROID_RUNTIME_DESCRIPTOR,create,destroy,transact);
    if(!type)return 73;
    AIBinder *binder=AIBinder_new(type,&state);if(!binder)return 74;
    state.api.request_sid(binder,true);
    int fd;
    if(state.api.setup_poll(&fd)!=STATUS_OK)return 75;
    if(state.api.add(binder,BNDROID_RUNTIME_SERVICE)!=EX_NONE)return 76;
    printf("RUNTIME_SERVICE_READY pid=%d uid=%u instance=%" PRIu64 "\n",getpid(),getuid(),state.instance);
    while(!state.stopping) {
        struct pollfd event={.fd=fd,.events=POLLIN};
        int result=poll(&event,1,-1);
        if(result<0 && errno==EINTR)continue;
        if(result!=1 || !(event.revents&POLLIN) || state.api.handle_poll()!=STATUS_OK)return 77;
    }
    // handlePolledCommands flushes the successful reply before returning.
    printf("RUNTIME_SERVICE_STOPPED instance=%" PRIu64 "\n",state.instance);
    AIBinder_decStrong(binder);return 0;
}
