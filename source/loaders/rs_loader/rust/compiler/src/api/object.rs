use super::*;
use std::{
    ffi::{c_void, CString},
    os::raw::{c_char, c_int},
};
#[repr(C)]
pub struct ObjectInterface {
    create: extern "C" fn(OpaqueType, OpaqueType) -> c_int,

    get: extern "C" fn(OpaqueType, OpaqueType, OpaqueType) -> OpaqueType,
    set: extern "C" fn(OpaqueType, OpaqueType, OpaqueType, OpaqueType) -> c_int,
    method_invoke:
        extern "C" fn(OpaqueType, OpaqueType, OpaqueType, OpaqueTypeList, usize) -> OpaqueType,
    method_await:
        extern "C" fn(OpaqueType, OpaqueType, OpaqueType, OpaqueTypeList, usize) -> OpaqueType,
    destructor: extern "C" fn(OpaqueType, OpaqueType),
    destroy: extern "C" fn(OpaqueType, OpaqueType),
}

#[no_mangle]
extern "C" fn object_singleton_create(_object: OpaqueType, _object_impl: OpaqueType) -> c_int {
    0
}

#[no_mangle]
extern "C" fn object_singleton_set(
    _object: OpaqueType,
    _object_impl: OpaqueType,
    _accessor: OpaqueType,
    _value: OpaqueType,
) -> c_int {
    0
}

#[no_mangle]
extern "C" fn object_singleton_get(
    _object: OpaqueType,
    _object_impl: OpaqueType,
    _accessor: OpaqueType,
) -> OpaqueType {
    0 as OpaqueType
}

#[no_mangle]
extern "C" fn object_singleton_method_invoke(
    _object: OpaqueType,
    _object_impl: OpaqueType,
    _method: OpaqueType,
    _args_p: OpaqueTypeList,
    _size: usize,
) -> OpaqueType {
    0 as OpaqueType
}

#[no_mangle]
extern "C" fn object_singleton_method_await(
    _object: OpaqueType,
    _object_impl: OpaqueType,
    _method: OpaqueType,
    _args_p: OpaqueTypeList,
    _size: usize,
) -> OpaqueType {
    0 as OpaqueType
}
#[no_mangle]
extern "C" fn object_singleton_destructor(_object: OpaqueType, _object_impl: OpaqueType) {}
#[no_mangle]
extern "C" fn object_singleton_destroy(_object: OpaqueType, _object_impl: OpaqueType) {}

#[no_mangle]
pub extern "C" fn object_singleton() -> *const ObjectInterface {
    static SINGLETON: ObjectInterface = ObjectInterface {
        create: object_singleton_create,
        get: object_singleton_get,
        set: object_singleton_set,
        method_invoke: object_singleton_method_invoke,
        method_await: object_singleton_method_await,
        destructor: object_singleton_destructor,
        destroy: object_singleton_destroy,
    };

    &SINGLETON
}
