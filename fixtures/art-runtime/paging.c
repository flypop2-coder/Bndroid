#define _GNU_SOURCE
#include <errno.h>
#include <linux/futex.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/random.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr,"ART_PAGING_FAILED line=%d errno=%d\n",__LINE__,errno); return 1; } } while (0)
static long long rss(void) {
    FILE *file=fopen("/proc/self/stat","r");
    if (!file) return -1;
    char line[2048];
    char *ok=fgets(line,sizeof(line),file);
    fclose(file);
    if (!ok) return -1;
    char *next=strrchr(line,')');
    if (!next) return -1;
    next+=4;
    unsigned long long value=0;
    for(int field=4;field<=24;++field) value=strtoull(next,&next,10);
    return (long long)value;
}
static long long nanos(struct timespec value) { return value.tv_sec*1000000000LL+value.tv_nsec; }
static void *burn_cpu(void *argument) {
    struct timespec start,now;
    if (clock_gettime(CLOCK_THREAD_CPUTIME_ID,&start)) return (void *)(uintptr_t)1;
    volatile unsigned long sum=0;
    do {
        for(int n=0;n<10000;++n) sum+=(unsigned)n;
        if (clock_gettime(CLOCK_THREAD_CPUTIME_ID,&now)) return (void *)(uintptr_t)1;
    } while(nanos(now)-nanos(start)<80000000);
    *(long long *)argument=nanos(now)-nanos(start);
    return NULL;
}
int art_paging_checks(void) {
    const size_t page=4096, pages=1024, size=page*pages;
    volatile unsigned char *bytes=mmap(NULL,size,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    CHECK(bytes!=MAP_FAILED);
    memset((void *)bytes,0x5a,size);
    long long before=rss(); CHECK(before>0);
    CHECK(madvise((void *)bytes,size,MADV_DONTNEED)==0);
    long long discarded=rss(); CHECK(discarded>0 && before-discarded>=(long long)pages-8);
    for(size_t n=0;n<size;n+=page) CHECK(bytes[n]==0 && bytes[n+page-1]==0);
    long long repopulated=rss(); CHECK(repopulated-discarded>=(long long)pages-8);
    CHECK(madvise((void *)bytes,size,MADV_DONTNEED)==0);
    // These pages are absent: exact kernel usercopy instructions must fault in
    // owned zero pages for both read and write directions, then retry.
    CHECK(syscall(SYS_futex,bytes,FUTEX_WAKE_PRIVATE,1,NULL,0,0)==0);
    CHECK(getrandom((void *)(bytes+page),32,0)==32);
    CHECK(bytes[page+32]==0);
    CHECK(mprotect((void *)(bytes+2*page),page,PROT_READ)==0);
    CHECK(madvise((void *)(bytes+2*page),page,MADV_DONTNEED)==0);
    CHECK(getrandom((void *)(bytes+2*page),1,0)==-1 && errno==EFAULT);
    CHECK(bytes[2*page]==0);
    CHECK(mprotect((void *)(bytes+3*page),page,PROT_NONE)==0);
    CHECK(madvise((void *)(bytes+3*page),page,MADV_DONTNEED)==0);
    CHECK(getrandom((void *)(bytes+3*page),1,0)==-1 && errno==EFAULT);
    CHECK(mprotect((void *)(bytes+3*page),page,PROT_READ|PROT_WRITE)==0 && bytes[3*page]==0);
    CHECK(madvise((void *)bytes,0,MADV_DONTNEED)==0);
    CHECK(madvise((void *)(bytes+1),page,MADV_DONTNEED)==-1 && errno==EINVAL);
    CHECK(madvise((void *)0x40080000,page,MADV_DONTNEED)==-1 && errno==ENOMEM);
    bytes[0]=0x7f;
    CHECK(munmap((void *)(bytes+page),page)==0);
    CHECK(madvise((void *)bytes,3*page,MADV_DONTNEED)==-1 && errno==ENOMEM && bytes[0]==0x7f);
    CHECK(munmap((void *)bytes,size)==0);
    struct timespec cpu0,cpu1,wall0,wall1,process;
    CHECK(clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu0)==0 && clock_gettime(CLOCK_MONOTONIC,&wall0)==0);
    volatile unsigned long sum=0;
    do { for(int n=0;n<10000;++n) sum+=(unsigned)n; CHECK(clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu1)==0); }
    while(nanos(cpu1)-nanos(cpu0)<10000000);
    CHECK(clock_gettime(CLOCK_PROCESS_CPUTIME_ID,&process)==0 && nanos(process)>=nanos(cpu1));
    CHECK(clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu0)==0);
    CHECK(usleep(100000)==0 && clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu1)==0 && clock_gettime(CLOCK_MONOTONIC,&wall1)==0);
    CHECK(nanos(cpu1)-nanos(cpu0)<50000000 && nanos(wall1)-nanos(wall0)>=100000000);
    // Reaped workers contribute to the process clock, not the leader's own
    // thread clock; otherwise pthread joins silently inflate main CPU time.
    struct timespec process_before;
    CHECK(clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu0)==0 && clock_gettime(CLOCK_PROCESS_CPUTIME_ID,&process_before)==0);
    long long worker_cpu=0; pthread_t worker; void *result=(void *)(uintptr_t)1;
    CHECK(pthread_create(&worker,NULL,burn_cpu,&worker_cpu)==0 && pthread_join(worker,&result)==0 && result==NULL);
    CHECK(clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu1)==0 && clock_gettime(CLOCK_PROCESS_CPUTIME_ID,&process)==0);
    CHECK(worker_cpu>=80000000 && nanos(cpu1)-nanos(cpu0)<40000000 && nanos(process)-nanos(process_before)>=worker_cpu);
    puts("ART_PAGING_OK discard=1 rss=1 zero_fault=1 usercopy=1 protections=1 atomic_failure=1 cpu_time=1");
    return 0;
}
