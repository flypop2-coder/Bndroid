#include "protocol.h"
#include <android/log.h>
#include <dlfcn.h>
#include <stdarg.h>

// Minimal platform callback ABI; SDK stubs do not ship libselinux headers.
// All union members in the platform header are function pointers.
union policy_callback {int (*log)(int,const char *,...);};
static int policy_log(int type,const char *format,...) {
    int priority=type==0?ANDROID_LOG_ERROR:(type==1 || type==3?ANDROID_LOG_WARN:ANDROID_LOG_INFO);
    va_list args;va_start(args,format);
    int result=__android_log_vprint(priority,"bndroid-runtime-policy",format,args);
    va_end(args);return result;
}

bool platform_api_open(struct platform_api *api) {
    void *binder=dlopen("libbinder_ndk.so",RTLD_NOW);
    void *policy=dlopen("libselinux.so",RTLD_NOW);
    if(!binder || !policy)return false;
#define LOAD(field, symbol) do {api->field=dlsym(binder,symbol);if(!api->field)return false;}while(0)
    LOAD(add,"AServiceManager_addService");
    LOAD(check,"AServiceManager_checkService");
    LOAD(setup_poll,"ABinderProcess_setupPolling");
    LOAD(handle_poll,"ABinderProcess_handlePolledCommands");
    LOAD(pool_max,"ABinderProcess_setThreadPoolMaxThreadCount");
    LOAD(request_sid,"AIBinder_setRequestingSid");
    LOAD(calling_sid,"AIBinder_getCallingSid");
#undef LOAD
    api->check_access=dlsym(policy,"selinux_check_access");
    void (*set_callback)(int,union policy_callback)=dlsym(policy,"selinux_set_callback");
    if(!api->check_access || !set_callback)return false;
    // Original libselinux may emit an audit message without a trailing newline.
    // Structured Android logging keeps each process/record boundary intact.
    set_callback(0,(union policy_callback){.log=policy_log});
    return true;
}
binder_status_t platform_status(AParcel *out,binder_exception_t exception) {
    AStatus *status=AStatus_fromExceptionCode(exception);
    if(!status)return STATUS_NO_MEMORY;
    binder_status_t result=AParcel_writeStatusHeader(out,status);
    AStatus_delete(status);return result;
}
