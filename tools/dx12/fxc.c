/* Compiles HLSL as wgpu's Direct3D 12 backend does with FXC: D3DCompile
 * from d3dcompiler_47.dll (the one beside this program, or the system's),
 * the shader model 5.1 profiles, D3DCOMPILE_ENABLE_STRICTNESS.
 *
 *   fxc.exe MANIFEST
 *
 * MANIFEST holds a line for each shader: its file, its entry point and its
 * profile (vs_5_1, ps_5_1), separated by tabs. For each, a line is printed:
 * "OK <file> <milliseconds>" or "FAIL <file> <milliseconds>: <FXC's
 * message, on one line>". Exits 0 when every shader compiled.
 *
 * Built by tools/dx12/fxc.sh with mingw-w64 and run under Wine there. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

typedef struct Blob Blob;
typedef struct BlobVtbl {
    HRESULT(WINAPI *QueryInterface)(Blob *, const void *, void **);
    ULONG(WINAPI *AddRef)(Blob *);
    ULONG(WINAPI *Release)(Blob *);
    void *(WINAPI *GetBufferPointer)(Blob *);
    SIZE_T(WINAPI *GetBufferSize)(Blob *);
} BlobVtbl;
struct Blob {
    const BlobVtbl *lpVtbl;
};

typedef HRESULT(WINAPI *D3DCompileFn)(const void *, SIZE_T, const char *, const void *, void *,
                                      const char *, const char *, UINT, UINT, Blob **, Blob **);

#define D3DCOMPILE_ENABLE_STRICTNESS (1 << 11)

static char *slurp(const char *path, long *size) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    *size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *data = malloc(*size + 1);
    if (fread(data, 1, *size, f) != (size_t)*size) {
        fclose(f);
        free(data);
        return NULL;
    }
    data[*size] = 0;
    fclose(f);
    return data;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: fxc.exe MANIFEST\n");
        return 2;
    }
    HMODULE lib = LoadLibraryA("d3dcompiler_47.dll");
    if (!lib) {
        fprintf(stderr, "no d3dcompiler_47.dll\n");
        return 2;
    }
    D3DCompileFn compile = (D3DCompileFn)GetProcAddress(lib, "D3DCompile");
    if (!compile) {
        fprintf(stderr, "no D3DCompile in d3dcompiler_47.dll\n");
        return 2;
    }
    long size = 0;
    char *manifest = slurp(argv[1], &size);
    if (!manifest) {
        fprintf(stderr, "can't read %s\n", argv[1]);
        return 2;
    }
    int failed = 0;
    LARGE_INTEGER freq;
    QueryPerformanceFrequency(&freq);
    for (char *line = strtok(manifest, "\r\n"); line; line = strtok(NULL, "\r\n")) {
        char *file = line;
        char *entry = strchr(file, '\t');
        if (!entry) continue;
        *entry++ = 0;
        char *profile = strchr(entry, '\t');
        if (!profile) continue;
        *profile++ = 0;
        /* (strtok's state is the manifest's: the shader read apart.) */
        long length = 0;
        char *source = slurp(file, &length);
        if (!source) {
            printf("FAIL %s 0: can't read it\n", file);
            failed++;
            continue;
        }
        Blob *code = NULL, *errors = NULL;
        LARGE_INTEGER t0, t1;
        QueryPerformanceCounter(&t0);
        HRESULT hr = compile(source, length, file, NULL, NULL, entry, profile,
                             D3DCOMPILE_ENABLE_STRICTNESS, 0, &code, &errors);
        QueryPerformanceCounter(&t1);
        long ms = (long)((t1.QuadPart - t0.QuadPart) * 1000 / freq.QuadPart);
        if (SUCCEEDED(hr)) {
            printf("OK %s %ld\n", file, ms);
        } else {
            failed++;
            printf("FAIL %s %ld: hr 0x%08lx", file, ms, (unsigned long)hr);
            if (errors) {
                const char *text = errors->lpVtbl->GetBufferPointer(errors);
                SIZE_T n = errors->lpVtbl->GetBufferSize(errors);
                printf(": ");
                for (SIZE_T i = 0; i < n && text[i]; i++) {
                    putchar(text[i] == '\n' || text[i] == '\r' ? ' ' : text[i]);
                }
            }
            printf("\n");
        }
        if (code) code->lpVtbl->Release(code);
        if (errors) errors->lpVtbl->Release(errors);
        free(source);
        fflush(stdout);
    }
    return failed ? 1 : 0;
}
