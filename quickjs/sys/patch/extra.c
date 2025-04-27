bool _QJS_SetCFuncMagic(JSValue func_obj, int magic){
    JSObject *p;
    if (JS_VALUE_GET_TAG(func_obj) != JS_TAG_OBJECT)
        return false;

    p = JS_VALUE_GET_OBJ(func_obj);
    p->u.cfunc.magic = magic;
    return true;
}

int _QJS_GetCFuncMagic(JSValue func_obj){
    JSObject *p;
    if (JS_VALUE_GET_TAG(func_obj) != JS_TAG_OBJECT)
        return -1;

    p = JS_VALUE_GET_OBJ(func_obj);
    return p->u.cfunc.magic;
}

int QJS_GetRefCount(JSValue v)
{
    int count=-1;
    if (JS_VALUE_HAS_REF_COUNT(v)) {
        JSRefCountHeader *p = (JSRefCountHeader *)JS_VALUE_GET_PTR(v);
        count=p->ref_count;
    }
    return count;
}
