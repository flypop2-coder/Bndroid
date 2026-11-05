#include <stdio.h>
static volatile _Thread_local int calls;
static volatile int ready;
__attribute__((constructor)) static void begin(void) { ready = 1; }
__attribute__((destructor)) static void end(void) { puts("DYNAMIC_PLUGIN_UNLOADED"); }
int bndroid_plugin_compute(int value) {
    return ready ? value + 36 + calls++ : -1;
}
