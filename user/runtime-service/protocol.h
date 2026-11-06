#ifndef BNDROID_RUNTIME_PROTOCOL_H
#define BNDROID_RUNTIME_PROTOCOL_H
#include <android/binder_ibinder.h>
#include <android/binder_parcel.h>
#include <android/binder_status.h>
#include <stdbool.h>
#include <stdint.h>

#define BNDROID_RUNTIME_SERVICE "bndroid.runtime"
#define BNDROID_RUNTIME_DESCRIPTOR "org.bndroid.runtime.IPlatformRuntime"
#define BNDROID_RUNTIME_CONTEXT "u:object_r:bndroid_runtime_service:s0"
#define BNDROID_RUNTIME_VERSION 1
#define BNDROID_RUNTIME_SNAPSHOT 1
#define BNDROID_RUNTIME_STOP 2

/* Platform APIs are resolved from the same verified libbinder_ndk as the
 * public parcel ABI; missing platform exports are a startup error. */
struct platform_api {
    binder_exception_t (*add)(AIBinder *, const char *);
    AIBinder *(*check)(const char *);
    binder_status_t (*setup_poll)(int *);
    binder_status_t (*handle_poll)(void);
    bool (*pool_max)(uint32_t);
    void (*request_sid)(AIBinder *, bool);
    const char *(*calling_sid)(void);
    int (*check_access)(const char *, const char *, const char *, const char *, void *);
};
bool platform_api_open(struct platform_api *api);
binder_status_t platform_status(AParcel *out, binder_exception_t exception);

#endif
