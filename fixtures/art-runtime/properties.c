#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/random.h>
#include <sys/system_properties.h>
#include <time.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr,"ART_PROPERTIES_FAILED line=%d errno=%d\n",__LINE__,errno); return 1; } } while (0)
struct property_value { int matched; uint32_t serial; };
static void read_value(void *cookie, const char *name, const char *value, uint32_t serial) {
    struct property_value *result = cookie;
    result->matched = strcmp(name, "ro.product.cpu.abilist64") == 0 &&
                      strcmp(value, "arm64-v8a") == 0 && (serial >> 24) == 9;
    result->serial = serial;
}
static void count_property(const prop_info *property, void *cookie) {
    if (property) ++*(int *)cookie;
}
int main(void) {
    char value[PROP_VALUE_MAX];
    CHECK(__system_property_get("ro.product.cpu.abilist64", value) == 9 && strcmp(value, "arm64-v8a") == 0);
    CHECK(__system_property_get("ro.product.name", value) == 7 && strcmp(value, "bndroid") == 0);
    CHECK(__system_property_get("ro.dalvik.vm.native.bridge", value) == 1 && strcmp(value, "0") == 0);
    CHECK(__system_property_get("ro.product.cpu.abilist32", value) == 0 && value[0] == 0);
    CHECK(__system_property_get("ro.bndroid.absent", value) == 0 && value[0] == 0);
    CHECK(__system_property_get("ro.build.version.sdk", value) == 2 && strcmp(value, "36") == 0);
    CHECK(__system_property_get("ro.build.version.sdk_full", value) == 4 && strcmp(value, "36.1") == 0);
    CHECK(__system_property_get("ro.build.version.all_codenames", value) == 3 && strcmp(value, "REL") == 0);
    const prop_info *property = __system_property_find("ro.product.cpu.abilist64");
    CHECK(property != NULL && __system_property_find("ro.bndroid.absent") == NULL);
    struct property_value result = {0, 0};
    int count = 0;
    __system_property_read_callback(property, read_value, &result);
    CHECK(result.matched == 1 && __system_property_foreach(count_property, &count) == 0 && count == 13);
    struct timespec before, after, monotonic;
    CHECK(clock_gettime(CLOCK_BOOTTIME, &before) == 0);
    struct timespec timeout = {0, 10000000};
    uint32_t next = 0;
    CHECK(!__system_property_wait(property, result.serial, &next, &timeout));
    CHECK(clock_gettime(CLOCK_BOOTTIME, &after) == 0 && clock_gettime(CLOCK_MONOTONIC, &monotonic) == 0);
    int64_t elapsed = (after.tv_sec-before.tv_sec)*1000000000LL + after.tv_nsec-before.tv_nsec;
    int64_t clocks = (monotonic.tv_sec-after.tv_sec)*1000000000LL + monotonic.tv_nsec-after.tv_nsec;
    CHECK(elapsed >= 10000000 && clocks >= 0 && clocks < 1000000000);
    int fd = open("/dev/__properties__", O_RDONLY|O_CLOEXEC);
    CHECK(fd >= 0);
    void *mapped = mmap(NULL, 128*1024, PROT_READ, MAP_SHARED, fd, 0);
    CHECK(mapped != MAP_FAILED && getrandom(mapped, 1, 0) == -1 && errno == EFAULT);
    CHECK(mprotect(mapped, 128*1024, PROT_READ|PROT_WRITE) == -1 && errno == EACCES);
    CHECK(mmap(NULL, 128*1024, PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0) == MAP_FAILED && errno == EACCES);
    CHECK(open("/dev/__properties__", O_RDWR) == -1 && errno == EROFS);
    CHECK(munmap(mapped, 128*1024) == 0 && close(fd) == 0);
    CHECK(__system_property_get("ro.product.cpu.abilist64", value) == 9 && strcmp(value, "arm64-v8a") == 0);
    puts("ART_PROPERTIES_OK bionic_reader=1 callback=1 enumerate=1 wait=1 immutable=1");
    return 0;
}
