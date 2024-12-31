This crate is a low level unsafe raw bindings for the QuickJS JavaScript engine.

## How to debug 'Assertion failed: (p->ref_count > 0), function gc_decref_child' ?
1. apply patch
```diff
diff --git a/quickjs.c b/quickjs.c
index 9cac6de..562b486 100644
--- a/quickjs.c
+++ b/quickjs.c
@@ -5829,6 +5829,9 @@ static void mark_children(JSRuntime *rt, JSGCObjectHeader *gp,

 static void gc_decref_child(JSRuntime *rt, JSGCObjectHeader *p)
 {
+    if (p->ref_count <= 0) {
+      JS_DumpGCObject(rt, p);
+    }
     assert(p->ref_count > 0);
     p->ref_count--;
     if (p->ref_count == 0 && p->mark == 1) {
```
2. run
```sh
DUMPFLAGS=1 cargo run

```
3. use address printed by JS_DumpGCObject to find which object had been freed.

