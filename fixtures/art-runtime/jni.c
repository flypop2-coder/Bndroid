#include <jni.h>
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>

static JavaVM *machine;
JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    (void)reserved;
    JNIEnv *env=NULL;
    if ((*vm)->GetEnv(vm,(void **)&env,JNI_VERSION_1_6)!=JNI_OK || !env) return JNI_ERR;
    machine=vm;
    return JNI_VERSION_1_6;
}
JNIEXPORT jint JNICALL Java_ManagedMain_mutate(JNIEnv *env,jclass type,jintArray array) {
    (void)type;
    jsize size=(*env)->GetArrayLength(env,array);
    jint *values=(*env)->GetIntArrayElements(env,array,NULL);
    if (!values) return -1;
    jint sum=0;
    for(jsize n=0;n<size;++n) { sum+=values[n]; values[n]+=10; }
    (*env)->ReleaseIntArrayElements(env,array,values,0);
    return sum;
}
struct Callback { jclass type; jmethodID method; jint result; };
static void *attached(void *opaque) {
    struct Callback *call=opaque;
    JNIEnv *env=NULL;
    if ((*machine)->AttachCurrentThread(machine,&env,NULL)!=JNI_OK) return (void *)(uintptr_t)1;
    call->result=(*env)->CallStaticIntMethod(env,call->type,call->method,41);
    int failed=(*env)->ExceptionCheck(env);
    if (failed) { (*env)->ExceptionDescribe(env); (*env)->ExceptionClear(env); }
    if ((*machine)->DetachCurrentThread(machine)!=JNI_OK) failed=1;
    return (void *)(uintptr_t)failed;
}
JNIEXPORT jint JNICALL Java_ManagedMain_attachedCallback(JNIEnv *env,jclass type) {
    struct Callback call={.type=(*env)->NewGlobalRef(env,type),.result=-1};
    if (!call.type) return -1;
    call.method=(*env)->GetStaticMethodID(env,type,"callback","(I)I");
    if (!call.method) { (*env)->DeleteGlobalRef(env,call.type); return -1; }
    pthread_t thread;
    void *failure=(void *)(uintptr_t)1;
    int status=pthread_create(&thread,NULL,attached,&call);
    if (!status) status=pthread_join(thread,&failure);
    (*env)->DeleteGlobalRef(env,call.type);
    return status || failure ? -1 : call.result;
}
JNIEXPORT void JNICALL Java_ManagedMain_reject(JNIEnv *env,jclass type) {
    (void)type;
    jclass exception=(*env)->FindClass(env,"java/lang/IllegalArgumentException");
    if (exception) (*env)->ThrowNew(env,exception,"native rejection");
}
JNIEXPORT jstring JNICALL Java_ManagedMain_nativePath(JNIEnv *env,jclass type) {
    (void)type;
    Dl_info location;
    if (!dladdr((void *)Java_ManagedMain_nativePath,&location) || !location.dli_fname) return NULL;
    return (*env)->NewStringUTF(env,location.dli_fname);
}
