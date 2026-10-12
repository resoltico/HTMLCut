#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <sys/resource.h>
#include <mach/mach.h>
#include <sys/mman.h>
extern int memorystatus_control(uint32_t, int32_t, uint32_t, void *, size_t);
struct memlimit { int32_t active; uint32_t active_attr; int32_t inactive; uint32_t inactive_attr; };
int main(int argc,char **argv) {
  void *reserved=NULL;
  if(argc>2 && strcmp(argv[2],"prereserve")==0) reserved=mmap(NULL,256*1024*1024,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANON,-1,0);
  struct mach_task_basic_info info; mach_msg_type_number_t count=MACH_TASK_BASIC_INFO_COUNT;
  task_info(mach_task_self(),MACH_TASK_BASIC_INFO,(task_info_t)&info,&count);
  printf("pid=%d uid=%d virtual=%llu resident=%llu\n",getpid(),getuid(),info.virtual_size,info.resident_size);
  if(argc>1 && strcmp(argv[1],"jetsam")==0) {
    struct memlimit lim={128,1,128,1};errno=0;
    int rc=memorystatus_control(7,getpid(),0,&lim,sizeof(lim));printf("memorystatus cmd=7 capMiB=128 rc=%d errno=%d\n",rc,errno);
  } else {
    rlim_t cap=argc>1 && strcmp(argv[1],"delta")==0 ? info.virtual_size+128*1024*1024 : 128*1024*1024;
    struct rlimit lim={cap,cap};errno=0;int kind=argc>2 && strcmp(argv[2],"data")==0?RLIMIT_DATA:RLIMIT_AS; int rc=setrlimit(kind,&lim);printf("setrlimit kind=%d cap=%llu rc=%d errno=%d\n",kind,cap,rc,errno);
  }
  fflush(stdout);
  if(reserved && reserved!=MAP_FAILED) {
    memset(reserved,42,256*1024*1024);
    count=MACH_TASK_BASIC_INFO_COUNT;task_info(mach_task_self(),MACH_TASK_BASIC_INFO,(task_info_t)&info,&count);
    printf("touched prereserveMiB=256 resident=%llu\n",info.resident_size);fflush(stdout);
  }
  if(argc>2 && strcmp(argv[2],"reset")==0) {
    struct rlimit lim={RLIM_INFINITY,RLIM_INFINITY};errno=0;int rc=setrlimit(RLIMIT_AS,&lim);printf("raise reset rc=%d errno=%d\n",rc,errno);fflush(stdout);
  }
  if(argc>2 && strcmp(argv[2],"reserve")==0) {
    size_t n=256*1024*1024;errno=0;void *mem=mmap(NULL,n,PROT_NONE,MAP_PRIVATE|MAP_ANON,-1,0);printf("untouched reserve rc=%p errno=%d\n",mem,errno);fflush(stdout);return mem==MAP_FAILED?3:0;
  }
  for(int i=0;i<12;i++){size_t n=32*1024*1024;void *mem=(argc>2 && strcmp(argv[2],"mmap")==0) ? mmap(NULL,n,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANON,-1,0) : malloc(n); if(mem==MAP_FAILED) mem=NULL;if(!mem){printf("allocation refused atMiB=%d errno=%d\n",i*32,errno);return 3;}memset(mem,42,n);count=MACH_TASK_BASIC_INFO_COUNT;task_info(mach_task_self(),MACH_TASK_BASIC_INFO,(task_info_t)&info,&count);printf("allocatedMiB=%d resident=%llu\n",(i+1)*32,info.resident_size);fflush(stdout);}
  return 0;
}
