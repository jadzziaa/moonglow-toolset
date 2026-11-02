/* An LD_PRELOAD shim (gcc -shared -fPIC -o libtimeshift.so timeshift.c -ldl;
 * keep the library outside /tmp, which the sandbox hides) that moves the
 * clocks a process reads: MG_SHIFT_REAL seconds added to the wall clock
 * (CLOCK_REALTIME and its coarse twin, gettimeofday, time), MG_SHIFT_MONO
 * seconds to the clocks counted from the boot (CLOCK_MONOTONIC, its raw
 * and coarse twins, CLOCK_BOOTTIME). For telling which clock the game
 * client's world timer (`worldtimerTimeOfDay`) starts from: run-client.sh
 * preloads it with MG_CLIENT_PRELOAD. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdlib.h>
#include <sys/time.h>
#include <time.h>

static long long shift(const char *name) {
    const char *v = getenv(name);
    return v ? atoll(v) : 0;
}

int clock_gettime(clockid_t id, struct timespec *ts) {
    static int (*real)(clockid_t, struct timespec *);
    if (!real) real = dlsym(RTLD_NEXT, "clock_gettime");
    int r = real(id, ts);
    if (r != 0 || !ts) return r;
    switch (id) {
    case CLOCK_REALTIME:
    case CLOCK_REALTIME_COARSE:
        ts->tv_sec += shift("MG_SHIFT_REAL");
        break;
    case CLOCK_MONOTONIC:
    case CLOCK_MONOTONIC_RAW:
    case CLOCK_MONOTONIC_COARSE:
    case CLOCK_BOOTTIME:
        ts->tv_sec += shift("MG_SHIFT_MONO");
        break;
    default:
        break;
    }
    return r;
}

int gettimeofday(struct timeval *tv, void *tz) {
    static int (*real)(struct timeval *, void *);
    if (!real) real = dlsym(RTLD_NEXT, "gettimeofday");
    int r = real(tv, tz);
    if (r == 0 && tv) tv->tv_sec += shift("MG_SHIFT_REAL");
    return r;
}

time_t time(time_t *t) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    if (t) *t = ts.tv_sec;
    return ts.tv_sec;
}
