/*
 * Opul's QuickJS driver, installed as Contents/Helpers/qjs.
 *
 * yt-dlp solves YouTube's JavaScript challenges by running `qjs --script <file>`.
 * Stock qjs always registers the `std` and `os` modules, which reach the file system
 * and can start processes. This driver evaluates the file as a classic script in a
 * context that has only print, console.log, performance.now and scriptArgs, and no
 * module loader, so `import('os')` fails. See docs/adr/0025.
 *
 * yt-dlp treats any output on stderr as a failure, so the driver writes to stderr
 * only when the script throws or leaves a promise rejection unhandled.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "quickjs-libc.h"

#define MEMORY_LIMIT (512 * 1024 * 1024)
#define STACK_SIZE (8 * 1024 * 1024)

static int unhandled_rejections;

static void track_rejection(JSContext *ctx, JSValueConst promise, JSValueConst reason,
                            JS_BOOL is_handled, void *opaque)
{
    unhandled_rejections += is_handled ? -1 : 1;
}

static int fail(JSContext *ctx)
{
    js_std_dump_error(ctx);
    return 1;
}

int main(int argc, char **argv)
{
    if (argc != 3 || strcmp(argv[1], "--script") != 0) {
        /* yt-dlp detects QuickJS from this line and expects --help to exit with 1. */
        printf("QuickJS version " CONFIG_VERSION "\n"
               "usage: qjs --script <file>\n");
        return 1;
    }

    JSRuntime *rt = JS_NewRuntime();
    JSContext *ctx = rt ? JS_NewContext(rt) : NULL;
    if (!ctx) {
        fputs("qjs: could not create a JavaScript context\n", stderr);
        return 1;
    }
    JS_SetMemoryLimit(rt, MEMORY_LIMIT);
    JS_SetMaxStackSize(rt, STACK_SIZE);
    JS_SetHostPromiseRejectionTracker(rt, track_rejection, NULL);

    js_std_add_helpers(ctx, 0, NULL);
    /* js_std_add_helpers also defines __loadScript, which reads and runs any file. */
    JSValue global = JS_GetGlobalObject(ctx);
    JSAtom load_script = JS_NewAtom(ctx, "__loadScript");
    JS_DeleteProperty(ctx, global, load_script, 0);
    JS_FreeAtom(ctx, load_script);
    JS_FreeValue(ctx, global);

    size_t length;
    uint8_t *source = js_load_file(ctx, &length, argv[2]);
    if (!source) {
        fprintf(stderr, "qjs: could not read %s\n", argv[2]);
        return 1;
    }
    JSValue result = JS_Eval(ctx, (const char *)source, length, argv[2], JS_EVAL_TYPE_GLOBAL);
    js_free(ctx, source);
    if (JS_IsException(result))
        return fail(ctx);
    JS_FreeValue(ctx, result);

    JSContext *job_context;
    int pending;
    while ((pending = JS_ExecutePendingJob(rt, &job_context)) != 0) {
        if (pending < 0)
            return fail(job_context);
    }
    fflush(stdout);
    if (unhandled_rejections > 0) {
        fputs("qjs: a promise rejection was not handled\n", stderr);
        return 1;
    }
    return 0;
}
