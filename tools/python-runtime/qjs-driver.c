/* Only this host entry point reads a script. No quickjs-libc, std, os,
 * module loader, or filesystem/process/network helpers are linked or exposed. */
#include "quickjs.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static int rejections;
static double deadline;

static double now(void) {
    struct timespec value;
    clock_gettime(CLOCK_MONOTONIC, &value);
    return value.tv_sec + value.tv_nsec / 1e9;
}

static int interrupt(JSRuntime *runtime, void *opaque) {
    return now() > deadline;
}

static JSValue print(JSContext *ctx, JSValueConst self, int argc, JSValueConst *argv) {
    for (int i = 0; i < argc; i++) {
        const char *text = JS_ToCString(ctx, argv[i]);
        if (!text) return JS_EXCEPTION;
        if (i) putchar(' ');
        fputs(text, stdout);
        JS_FreeCString(ctx, text);
    }
    putchar('\n');
    return JS_UNDEFINED;
}

static JSValue performance_now(JSContext *ctx, JSValueConst self, int argc, JSValueConst *argv) {
    return JS_NewFloat64(ctx, now() * 1000);
}

static void rejection(JSContext *ctx, JSValueConst promise, JSValueConst reason,
                      JS_BOOL handled, void *opaque) {
    rejections += handled ? -1 : 1;
}

static void dump_error(JSContext *ctx) {
    JSValue error = JS_GetException(ctx);
    const char *text = JS_ToCString(ctx, error);
    fprintf(stderr, "qjs: %s\n", text ? text : "JavaScript exception");
    JS_FreeCString(ctx, text);
    JS_FreeValue(ctx, error);
}

int main(int argc, char **argv) {
    if (argc != 3 || strcmp(argv[1], "--script")) {
        puts("QuickJS version " CONFIG_VERSION "\nusage: qjs --script <file>");
        return 1; /* yt-dlp expects --help to return 1. */
    }
    FILE *file = fopen(argv[2], "rb");
    if (!file) { perror("qjs"); return 1; }
    if (fseek(file, 0, SEEK_END)) { fclose(file); return 1; }
    long length = ftell(file);
    if (length < 0 || length > 32 * 1024 * 1024) { fclose(file); return 1; }
    rewind(file);
    char *source = malloc((size_t)length + 1);
    if (!source) { fclose(file); return 1; }
    size_t read = fread(source, 1, (size_t)length, file);
    fclose(file);
    if (read != (size_t)length) { free(source); return 1; }
    source[length] = 0;

    JSRuntime *runtime = JS_NewRuntime();
    if (!runtime) { free(source); return 1; }
    JS_SetMemoryLimit(runtime, 512 * 1024 * 1024);
    JS_SetMaxStackSize(runtime, 8 * 1024 * 1024);
    deadline = now() + 30;
    JS_SetInterruptHandler(runtime, interrupt, NULL);
    JS_SetHostPromiseRejectionTracker(runtime, rejection, NULL);
    JSContext *ctx = JS_NewContext(runtime);
    if (!ctx) { free(source); JS_FreeRuntime(runtime); return 1; }
    JSValue global = JS_GetGlobalObject(ctx);
    JS_SetPropertyStr(ctx, global, "print", JS_NewCFunction(ctx, print, "print", 1));
    JSValue console = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, console, "log", JS_NewCFunction(ctx, print, "log", 1));
    JS_SetPropertyStr(ctx, global, "console", console);
    JSValue performance = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, performance, "now", JS_NewCFunction(ctx, performance_now, "now", 0));
    JS_SetPropertyStr(ctx, global, "performance", performance);
    JS_SetPropertyStr(ctx, global, "scriptArgs", JS_NewArray(ctx));
    JS_FreeValue(ctx, global);
    JSValue result = JS_Eval(ctx, source, (size_t)length, "challenge.js", JS_EVAL_TYPE_GLOBAL);
    free(source);
    int status = JS_IsException(result);
    JS_FreeValue(ctx, result);
    if (status) dump_error(ctx);
    JSContext *job_ctx;
    while (!status && JS_IsJobPending(runtime)) {
        if (JS_ExecutePendingJob(runtime, &job_ctx) < 0) {
            dump_error(job_ctx);
            status = 1;
        }
    }
    if (rejections > 0) {
        fputs("qjs: unhandled promise rejection\n", stderr);
        status = 1;
    }
    JS_FreeContext(ctx);
    JS_FreeRuntime(runtime);
    return status;
}
