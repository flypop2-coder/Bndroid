#include "protocol.h"
#include <dlfcn.h>

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
    return api->check_access!=NULL;
}
binder_status_t platform_status(AParcel *out,binder_exception_t exception) {
    AStatus *status=AStatus_fromExceptionCode(exception);
    if(!status)return STATUS_NO_MEMORY;
    binder_status_t result=AParcel_writeStatusHeader(out,status);
    AStatus_delete(status);return result;
}
