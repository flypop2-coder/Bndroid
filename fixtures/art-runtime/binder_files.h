/* Owned cross-process FD and Binder-poll probe, included after the wire helpers. */
static void start_binder_poll(int fd) {
    polled_binder=fd;
    binder_poll=epoll_create1(EPOLL_CLOEXEC);binder_outer=epoll_create1(EPOLL_CLOEXEC);
    CHECK(binder_poll>=0 && binder_outer>=0 && fcntl(fd,F_SETFL,O_NONBLOCK)==0);
    struct epoll_event event={.events=EPOLLIN|EPOLLET,.data.u64=0xb1};
    CHECK(epoll_ctl(binder_poll,EPOLL_CTL_ADD,fd,&event)==0);
    event.data.u64=0xb2;CHECK(epoll_ctl(binder_outer,EPOLL_CTL_ADD,binder_poll,&event)==0);
}
static void stop_binder_poll(void) {
    CHECK(binder_poll>=0 && binder_outer>=0 && close(binder_poll)==0 && close(binder_outer)==0);
    binder_poll=binder_outer=polled_binder=-1;
}
static struct flat_binder_object fd_object(int fd) {
    return (struct flat_binder_object){.hdr.type=BINDER_TYPE_FD,.handle=(uint32_t)fd,.cookie=0xdeadbeef};
}
static void send_files(int fd,uint32_t operation,uint32_t target,uint32_t code,uint32_t flags,const int *files,size_t count) {
    struct flat_binder_object objects[2];CHECK(count<=2);
    for(size_t i=0;i<count;++i) objects[i]=fd_object(files[i]);
    transmit_objects(fd,operation,target,code,flags,objects,count);
}
static int received_file(const struct event *event,size_t index,size_t count) {
    struct flat_binder_object o=get_object(event,index,count);
    CHECK(o.hdr.type==BINDER_TYPE_FD && o.binder==o.handle && o.handle<0x7fffffff && o.cookie==0xdeadbeef);
    int fd=(int)o.handle;CHECK(fcntl(fd,F_GETFD)==FD_CLOEXEC);return fd;
}
static int memory_file(void) {
    int fd=(int)syscall(__NR_memfd_create,"binder-shared",MFD_ALLOW_SEALING);CHECK(fd>=0);return fd;
}
static int fill_table(int *fds,size_t *count) {
    *count=0;
    for(;;) {
        int fd=open("/dev/null",O_RDONLY);
        if(fd<0) {CHECK(errno==EMFILE);break;}
        CHECK(*count<600);fds[(*count)++]=fd;
    }
    CHECK(*count>0);int hole=fds[--*count];CHECK(close(hole)==0);return hole;
}
static void check_hole_and_release(int hole,const int *fds,size_t count) {
    int fd=open("/dev/null",O_RDONLY);CHECK(fd==hole);
    CHECK(open("/dev/null",O_RDONLY)==-1 && errno==EMFILE);
    CHECK(close(fd)==0);for(size_t i=0;i<count;++i) CHECK(close(fds[i])==0);
}
static void expect_complete(int fd) {
    unsigned char bytes[256];struct binder_write_read io={.read_size=sizeof(bytes),.read_buffer=(uintptr_t)bytes};
    CHECK(ioctl(fd,BINDER_WRITE_READ,&io)==0 && io.read_consumed==8);
    uint32_t code;memcpy(&code,bytes+4,4);CHECK(code==BR_TRANSACTION_COMPLETE);
}
static void *unrelated_poll_thread(void *unused) {
    (void)unused;struct epoll_event event;
    CHECK(epoll_wait(binder_outer,&event,1,0)==0);
    return NULL;
}
static void files_client(int fd) {
    start_binder_poll(fd);
    int original[2]={memory_file(),eventfd(0,EFD_NONBLOCK)};CHECK(original[1]>=0);
    CHECK(fcntl(original[0],F_GETFD)==0 && ftruncate(original[0],8192)==0);
    unsigned char *mapped=mmap(NULL,8192,PROT_READ|PROT_WRITE,MAP_SHARED,original[0],0);CHECK(mapped!=MAP_FAILED);
    uint64_t magic=0x1122334455667788ULL;memcpy(mapped,&magic,8);mapped[4096]=0xaa;
    CHECK(lseek(original[0],7,SEEK_SET)==7);
    send_files(fd,BC_TRANSACTION,0,70,TF_ACCEPT_FDS,original,2);
    CHECK(close(original[0])==0 && close(original[1])==0);
    int reused[2]={open("/dev/null",O_RDONLY),open("/dev/null",O_RDONLY)};
    CHECK(reused[0]==original[0] && reused[1]==original[1]);
    pthread_t probe;CHECK(pthread_create(&probe,NULL,unrelated_poll_thread,NULL)==0 && pthread_join(probe,NULL)==0);
    struct event response=receive(fd);CHECK(response.command==BR_REPLY);
    int files[2]={received_file(&response,0,2),received_file(&response,1,2)};release_buffer(fd,&response);
    CHECK(files[0]!=original[0] && mapped[0]==0x99 && lseek(files[0],0,SEEK_CUR)==8);
    CHECK(fcntl(files[1],F_GETFL)&O_NONBLOCK);uint64_t wake=0;CHECK(read(files[1],&wake,8)==8 && wake==9);
    struct stat info;CHECK(fstat(files[0],&info)==0 && info.st_size==4096);
    fd_fault_address=mapped+4096;
    __asm__ volatile("ldrb wzr, [%0]" :: "r"(fd_fault_address) : "memory");CHECK(fd_faults==1);
    CHECK(pwrite(files[0],"Z",1,8207)==1 && mapped[4096]==0);mapped[4099]=0x5a;
    CHECK(munmap(mapped,8192)==0 && close(reused[0])==0 && close(reused[1])==0);
    send_files(fd,BC_TRANSACTION,0,71,0,NULL,0);response=receive(fd);CHECK(response.command==BR_REPLY);
    uint32_t rejecting=retain_object(fd,get_object(&response,0,1),1);release_buffer(fd,&response);
    CHECK(pwrite(files[0],"x",1,0)==-1 && errno==EPERM);
    CHECK(mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_SHARED,files[0],0)==MAP_FAILED && errno==EPERM);
    send_files(fd,BC_TRANSACTION,rejecting,71,0,files,1);CHECK(receive(fd).command==BR_FAILED_REPLY);
    int invalid=9999;send_files(fd,BC_TRANSACTION,0,71,0,&invalid,1);CHECK(receive(fd).command==BR_FAILED_REPLY);
    command(fd,BC_RELEASE,&rejecting,4);command(fd,BC_DECREFS,&rejecting,4);
    // Replies require the original caller's TF_ACCEPT_FDS capability.
    send_files(fd,BC_TRANSACTION,0,72,0,NULL,0);CHECK(receive(fd).command==BR_FAILED_REPLY);
    send_files(fd,BC_TRANSACTION,0,73,TF_ACCEPT_FDS,NULL,0);
    int filler[600];size_t count;int hole=fill_table(filler,&count);
    CHECK(receive(fd).command==BR_FAILED_REPLY);check_hole_and_release(hole,filler,count);
    // The peer now fills its table, leaving one slot for a two-FD request.
    send_files(fd,BC_TRANSACTION,0,74,0,NULL,0);response=receive(fd);CHECK(response.command==BR_REPLY);release_buffer(fd,&response);
    send_files(fd,BC_TRANSACTION,0,75,0,files,2);CHECK(receive(fd).command==BR_FAILED_REPLY);
    send_files(fd,BC_TRANSACTION,0,76,0,NULL,0);response=receive(fd);CHECK(response.command==BR_REPLY);release_buffer(fd,&response);
    send_files(fd,BC_TRANSACTION,0,77,0,files,2);CHECK(receive(fd).command==BR_FAILED_REPLY);
    send_files(fd,BC_TRANSACTION,0,78,TF_ACCEPT_FDS,NULL,0);response=receive(fd);CHECK(response.command==BR_REPLY);
    survivor_fd=received_file(&response,0,1);release_buffer(fd,&response);
    survivor_map=mmap(NULL,4096,PROT_READ,MAP_SHARED,survivor_fd,0);CHECK(survivor_map!=MAP_FAILED);
    CHECK(close(files[0])==0 && close(files[1])==0);
}
static void files_server(int fd,const struct event *request) {
    int files[2]={received_file(request,0,2),received_file(request,1,2)};release_buffer(fd,request);
    start_binder_poll(fd);CHECK(lseek(files[0],0,SEEK_CUR)==7);
    unsigned char byte;CHECK(read(files[0],&byte,1)==1 && byte==0x11);
    unsigned char *mapped=mmap(NULL,8192,PROT_READ|PROT_WRITE,MAP_SHARED,files[0],0);CHECK(mapped!=MAP_FAILED);
    CHECK(mapped[4096]==0xaa);mapped[0]=0x99;CHECK(munmap(mapped,8192)==0);
    // Only the other process has a writable-capable alias at this point.
    CHECK(fcntl(files[0],F_ADD_SEALS,F_SEAL_WRITE)==-1 && errno==EBUSY);
    CHECK(ftruncate(files[0],4096)==0);
    const unsigned char *reader=mmap(NULL,12288,PROT_READ,MAP_SHARED,files[0],0);CHECK(reader!=MAP_FAILED);
    uint64_t wake=9;CHECK(write(files[1],&wake,8)==8);
    send_files(fd,BC_REPLY,0,70,0,files,2);
    struct event next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==71);
    CHECK(reader[4099]==0x5a && reader[8207]=='Z' && munmap((void *)reader,12288)==0);
    CHECK(fcntl(files[0],F_ADD_SEALS,F_SEAL_WRITE|F_SEAL_SEAL)==0);release_buffer(fd,&next);
    struct flat_binder_object rejecting=local_object(SERVER_PTR+0x200,1);rejecting.flags=0;
    transmit_objects(fd,BC_REPLY,0,71,0,&rejecting,1);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==72);release_buffer(fd,&next);
    send_files(fd,BC_REPLY,0,72,0,files,1);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==73);release_buffer(fd,&next);
    send_files(fd,BC_REPLY,0,73,0,files,2);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==74);release_buffer(fd,&next);
    int filler[600];size_t count;int hole=fill_table(filler,&count);
    CHECK(fcntl(fd,F_SETFL,0)==0);send_files(fd,BC_REPLY,0,74,0,NULL,0);
    // A failed request install must return to the blocking receive path. Its
    // caller gets BR_FAILED_REPLY; this thread later receives request 76.
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==76);release_buffer(fd,&next);
    check_hole_and_release(hole,filler,count);
    send_files(fd,BC_REPLY,0,76,0,NULL,0);expect_complete(fd);
    struct binder_write_read invalid={.read_size=256,.read_buffer=0x40080000};
    CHECK(ioctl(fd,BINDER_WRITE_READ,&invalid)==-1 && errno==EFAULT && invalid.read_consumed==0);
    CHECK(fcntl(fd,F_SETFL,O_NONBLOCK)==0);
    next=receive(fd);CHECK(next.command==BR_TRANSACTION && next.transaction.code==78);release_buffer(fd,&next);
    int survivor=memory_file();CHECK(pwrite(survivor,"binder survives",15,0)==15);
    send_files(fd,BC_REPLY,0,78,0,&survivor,1);
    CHECK(close(survivor)==0 && close(files[0])==0 && close(files[1])==0 && binder_poll_wakes>0);
    puts("BINDER_FD_SERVER_OK accept=1 capture=1 full=1 copy_fault=1 cross_process_maps=1 poll=1");
}
