#define _GNU_SOURCE
#include "../../user/runtime-service/protocol.h"
#include <errno.h>
#include <inttypes.h>
#include <poll.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/system_properties.h>
#include <sys/utsname.h>
#include <time.h>
#include <unistd.h>

#define CHECK(x) do {if(!(x)){fprintf(stderr,"SERVICE_BRIDGE_FAIL line=%d errno=%d uid=%u\n",__LINE__,errno,getuid());exit(81);}}while(0)
static atomic_bool done;
static void *watchdog(void *unused) {
    (void)unused;for(int i=0;i<2000;++i){if(atomic_load(&done))return NULL;usleep(10000);}
    fputs("SERVICE_BRIDGE_TIMEOUT\n",stderr);_exit(82);
}
static void *create(void *state){return state;}
static void destroy(void *state){(void)state;}
static binder_status_t unused_call(AIBinder *b,transaction_code_t c,const AParcel *in,AParcel *out){(void)b;(void)c;(void)in;(void)out;return STATUS_UNKNOWN_TRANSACTION;}
static AParcel *call(AIBinder *binder,int code,int version,int extra,binder_exception_t exception) {
    AParcel *in=NULL,*out=NULL;CHECK(AIBinder_prepareTransaction(binder,&in)==STATUS_OK);
    CHECK(AParcel_writeInt32(in,version)==STATUS_OK);
    if(extra)CHECK(AParcel_writeInt32(in,99)==STATUS_OK);
    CHECK(AIBinder_transact(binder,code,&in,&out,0)==STATUS_OK && in==NULL && out);
    AStatus *status=NULL;CHECK(AParcel_readStatusHeader(out,&status)==STATUS_OK && status);
    CHECK(AStatus_getExceptionCode(status)==exception);AStatus_delete(status);return out;
}
static bool string_buffer(void *data,int32_t length,char **buffer) {
    if(length<=0 || length>128)return false;
    *buffer=data;return true;
}
static uint64_t snapshot(AIBinder *binder) {
    AParcel *out=call(binder,BNDROID_RUNTIME_SNAPSHOT,1,0,EX_NONE);
    int32_t version,pid,uid,caller_pid,caller_uid;int64_t uptime;uint64_t instance;
    CHECK(AParcel_readInt32(out,&version)==STATUS_OK && version==1);
    CHECK(AParcel_readUint64(out,&instance)==STATUS_OK && instance!=0);
    CHECK(AParcel_readInt64(out,&uptime)==STATUS_OK && uptime>0);
    CHECK(AParcel_readInt32(out,&pid)==STATUS_OK && pid>0 && pid!=getpid());
    CHECK(AParcel_readInt32(out,&uid)==STATUS_OK && uid==1001);
    CHECK(AParcel_readInt32(out,&caller_pid)==STATUS_OK && caller_pid==getpid());
    CHECK(AParcel_readInt32(out,&caller_uid)==STATUS_OK && caller_uid==(int32_t)getuid());
    struct utsname system;struct timespec now;CHECK(uname(&system)==0 && clock_gettime(CLOCK_BOOTTIME,&now)==0);
    CHECK(uptime<=(int64_t)now.tv_sec*1000000000+now.tv_nsec);
    char name[128],release[128],machine[128];
    CHECK(AParcel_readString(out,name,string_buffer)==STATUS_OK && strcmp(name,system.nodename)==0);
    CHECK(AParcel_readString(out,release,string_buffer)==STATUS_OK && strcmp(release,system.release)==0);
    CHECK(AParcel_readString(out,machine,string_buffer)==STATUS_OK && strcmp(machine,system.machine)==0);
    CHECK(AParcel_getDataPosition(out)==AParcel_getDataSize(out));AParcel_delete(out);return instance;
}
static AIBinder *discover(struct platform_api *api,AIBinder_Class *type) {
    AIBinder *binder=NULL;
    for(int n=0;n<1000 && !binder;++n) {
        binder=api->check(BNDROID_RUNTIME_SERVICE);
        if(binder) {
            binder_status_t status=AIBinder_ping(binder);
            CHECK(status==STATUS_OK || status==STATUS_DEAD_OBJECT);
            // The manager and this client receive independent death events.
            // A stale registration can briefly outlive our own notification.
            if(status==STATUS_DEAD_OBJECT){AIBinder_decStrong(binder);binder=NULL;}
        }
        if(!binder)usleep(5000);
    }
    CHECK(binder && AIBinder_associateClass(binder,type));return binder;
}
static void wait_property(const char *name) {
    char ready[PROP_VALUE_MAX]={0};
    for(int i=0;i<1000;++i){if(__system_property_get(name,ready)==4 && strcmp(ready,"true")==0)return;usleep(5000);}
    CHECK(false);
}
static unsigned deaths;
static void died(void *cookie){CHECK(cookie==&deaths);++deaths;}
int main(int argc,char **argv) {
    CHECK(argc==2 && setvbuf(stdout,NULL,_IOLBF,0)==0);
    bool controller=strcmp(argv[1],"controller")==0;CHECK(controller || strcmp(argv[1],"client")==0);
    CHECK(getuid()==(controller?1002:10000));
    pthread_t timeout;CHECK(pthread_create(&timeout,NULL,watchdog,NULL)==0);
    struct platform_api api;CHECK(platform_api_open(&api) && api.pool_max(0));
    int fd;CHECK(api.setup_poll(&fd)==STATUS_OK);
    AIBinder_Class *type=AIBinder_Class_define(BNDROID_RUNTIME_DESCRIPTOR,create,destroy,unused_call);CHECK(type);
    AIBinder *binder=discover(&api,type);uint64_t first=snapshot(binder);
    wait_property("servicemanager.ready");
    if(controller) {
        wait_property("bndroid.runtime.client_ready");
        AParcel *out=call(binder,BNDROID_RUNTIME_STOP,1,0,EX_NONE);AParcel_delete(out);
        printf("SERVICE_BRIDGE_CONTROLLER_OK pid=%d uid=%u instance=%" PRIu64 " control=authorized\n",getpid(),getuid(),first);
    } else {
        AIBinder *spoof=AIBinder_new(type,NULL);CHECK(spoof);
        CHECK(api.add(spoof,BNDROID_RUNTIME_SERVICE)==EX_SECURITY);AIBinder_decStrong(spoof);
        AParcel *denied=call(binder,BNDROID_RUNTIME_STOP,1,0,EX_SECURITY);AParcel_delete(denied);
        AParcel *bad=call(binder,BNDROID_RUNTIME_SNAPSHOT,99,0,EX_ILLEGAL_ARGUMENT);AParcel_delete(bad);
        bad=call(binder,BNDROID_RUNTIME_SNAPSHOT,1,1,EX_ILLEGAL_ARGUMENT);AParcel_delete(bad);
        AIBinder_DeathRecipient *recipient=AIBinder_DeathRecipient_new(died);CHECK(recipient);
        CHECK(AIBinder_linkToDeath(binder,recipient,&deaths)==STATUS_OK);
        CHECK(__system_property_set("bndroid.runtime.client_ready","true")==0);
        for(int n=0;n<500 && deaths==0;++n) {
            struct pollfd event={.fd=fd,.events=POLLIN};int result=poll(&event,1,10);
            CHECK(result>=0);if(result>0)CHECK(api.handle_poll()==STATUS_OK);
        }
        CHECK(deaths==1 && AIBinder_ping(binder)==STATUS_DEAD_OBJECT && !AIBinder_isAlive(binder));
        AIBinder *replacement=discover(&api,type);uint64_t second=snapshot(replacement);
        CHECK(second!=first && replacement!=binder && AIBinder_ping(replacement)==STATUS_OK);
        CHECK(AIBinder_ping(binder)==STATUS_DEAD_OBJECT && deaths==1);
        AIBinder_decStrong(replacement);AIBinder_DeathRecipient_delete(recipient);
        printf("SERVICE_BRIDGE_CLIENT_OK pid=%d uid=%u first=%" PRIu64 " second=%" PRIu64 " facts=1 identity=1 add_denied=1 stop_denied=1 malformed=1 death=1 stale=1 rediscovery=1 daemon_ready=1\n",getpid(),getuid(),first,second);
    }
    AIBinder_decStrong(binder);atomic_store(&done,true);CHECK(pthread_join(timeout,NULL)==0);return 0;
}
