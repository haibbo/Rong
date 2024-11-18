JS_BOOL _QJS_SetCFuncMagic(JSValue func_obj, int magic){
    JSObject *p;
    if (JS_VALUE_GET_TAG(func_obj) != JS_TAG_OBJECT)
        return FALSE;

    p = JS_VALUE_GET_OBJ(func_obj);
    p->u.cfunc.magic = magic;
    return TRUE;
}

int _QJS_GetCFuncMagic(JSValue func_obj){
    JSObject *p;
    if (JS_VALUE_GET_TAG(func_obj) != JS_TAG_OBJECT)
        return -1;

    p = JS_VALUE_GET_OBJ(func_obj);
    return p->u.cfunc.magic;
}
