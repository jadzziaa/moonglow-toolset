/* An LD_PRELOAD shim (gcc -shared -fPIC -o libexeclog.so execlog.c -ldl;
 * keep the library outside /tmp, which the sandbox hides) that writes every program a process starts (execve,
 * execv, execvp, posix_spawn, posix_spawnp, system) with its arguments to
 * the file named by MG_EXEC_LOG, then does what was asked. For reading what
 * the game client restarts itself with after an NWSync download. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static void note(const char *how, const char *path, char *const argv[]) {
    const char *log = getenv("MG_EXEC_LOG");
    if (!log) return;
    FILE *f = fopen(log, "a");
    if (!f) return;
    fprintf(f, "%s pid %d: %s", how, (int)getpid(), path ? path : "(null)");
    for (int i = 0; argv && argv[i]; i++) fprintf(f, " [%s]", argv[i]);
    fprintf(f, "\n");
    fclose(f);
}

int execve(const char *path, char *const argv[], char *const envp[]) {
    note("execve", path, argv);
    int (*real)(const char *, char *const[], char *const[]) = dlsym(RTLD_NEXT, "execve");
    return real(path, argv, envp);
}

int execv(const char *path, char *const argv[]) {
    note("execv", path, argv);
    int (*real)(const char *, char *const[]) = dlsym(RTLD_NEXT, "execv");
    return real(path, argv);
}

int execvp(const char *file, char *const argv[]) {
    note("execvp", file, argv);
    int (*real)(const char *, char *const[]) = dlsym(RTLD_NEXT, "execvp");
    return real(file, argv);
}

int execvpe(const char *file, char *const argv[], char *const envp[]) {
    note("execvpe", file, argv);
    int (*real)(const char *, char *const[], char *const[]) = dlsym(RTLD_NEXT, "execvpe");
    return real(file, argv, envp);
}

int posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *fa,
                const posix_spawnattr_t *attr, char *const argv[], char *const envp[]) {
    note("posix_spawn", path, argv);
    int (*real)(pid_t *, const char *, const posix_spawn_file_actions_t *,
                const posix_spawnattr_t *, char *const[], char *const[]) =
        dlsym(RTLD_NEXT, "posix_spawn");
    return real(pid, path, fa, attr, argv, envp);
}

int posix_spawnp(pid_t *pid, const char *file, const posix_spawn_file_actions_t *fa,
                 const posix_spawnattr_t *attr, char *const argv[], char *const envp[]) {
    note("posix_spawnp", file, argv);
    int (*real)(pid_t *, const char *, const posix_spawn_file_actions_t *,
                const posix_spawnattr_t *, char *const[], char *const[]) =
        dlsym(RTLD_NEXT, "posix_spawnp");
    return real(pid, file, fa, attr, argv, envp);
}

int system(const char *command) {
    char *argv[] = {(char *)command, NULL};
    note("system", "/bin/sh -c", argv);
    int (*real)(const char *) = dlsym(RTLD_NEXT, "system");
    return real(command);
}
