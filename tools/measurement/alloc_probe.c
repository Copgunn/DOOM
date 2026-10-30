/* GPL-2.0-only. Linux/glibc host measurement, never linked into the engine. */
#define _GNU_SOURCE
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>
extern void *__libc_malloc(size_t);
extern void *__libc_calloc(size_t,size_t);
extern void *__libc_realloc(void *,size_t);
extern void __libc_free(void *);
extern void *__libc_memalign(size_t,size_t);
static _Thread_local int active;
static _Thread_local uint64_t allocations,reallocations,bytes,frees,failures;
static void record(void *p,size_t n){if(active){if(p){++allocations;bytes+=n;}else if(n){++failures;}}}
void *malloc(size_t n){void *p=__libc_malloc(n);record(p,n);return p;}
void *calloc(size_t n,size_t s){void *p=__libc_calloc(n,s);if(s&&n>SIZE_MAX/s){if(active){++failures;}}else{record(p,n*s);}return p;}
void *realloc(void *old,size_t n){void *p=__libc_realloc(old,n);if(active){++reallocations;if(p||!n){bytes+=n;}else{++failures;}}return p;}
void free(void *p){if(active&&p){++frees;}__libc_free(p);}
void *aligned_alloc(size_t a,size_t n){void *p=__libc_memalign(a,n);record(p,n);return p;}
int posix_memalign(void **out,size_t a,size_t n){int saved=errno;void *p;if(a<sizeof(void*)||a%sizeof(void*)||(a&(a-1))){return EINVAL;}p=__libc_memalign(a,n);record(p,n);errno=saved;if(!p){return ENOMEM;}*out=p;return 0;}
static void marker(int fd,const void *p,size_t n){
 const char begin[]="DOOM_ALLOC_BEGIN\n",end[]="DOOM_ALLOC_END\n";
 if(fd!=STDOUT_FILENO){return;}
 if(n==sizeof(begin)-1&&!memcmp(p,begin,n)){allocations=reallocations=bytes=frees=failures=0;active=1;}
 else if(n==sizeof(end)-1&&!memcmp(p,end,n)){
  char line[256];int len;active=0;
  len=snprintf(line,sizeof(line),"ALLOC calls=%llu reallocs=%llu requested_bytes=%llu frees=%llu failures=%llu\n",(unsigned long long)allocations,(unsigned long long)reallocations,(unsigned long long)bytes,(unsigned long long)frees,(unsigned long long)failures);
  if(len>0&&(size_t)len<sizeof(line)){(void)syscall(SYS_write,STDERR_FILENO,line,(size_t)len);}
 }
}
ssize_t write(int fd,const void *p,size_t n){int saved=errno;marker(fd,p,n);errno=saved;return syscall(SYS_write,fd,p,n);}
ssize_t writev(int fd,const struct iovec *v,int n){int saved=errno;if(n==1){marker(fd,v[0].iov_base,v[0].iov_len);}errno=saved;return syscall(SYS_writev,fd,v,n);}
