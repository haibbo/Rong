#ifndef QJS_H
#define QJS_H

JSValue QJS_RunScript(JSContext *ctx, char *script, int len);
void QJS_RunJobs(JSRuntime *rt);

#endif
