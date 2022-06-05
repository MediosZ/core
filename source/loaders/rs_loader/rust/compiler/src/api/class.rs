use super::*;
use crate::Class;
use std::{
    ffi::{c_void, CString},
    os::raw::{c_char, c_int},
};
#[repr(C)]
pub struct ClassInterface {
    create: extern "C" fn(OpaqueType, OpaqueType) -> c_int,
    constructor: extern "C" fn(
        OpaqueType,
        OpaqueType,
        *const char,
        OpaqueType,
        OpaqueTypeList,
        usize,
    ) -> OpaqueType,
    static_get: extern "C" fn(OpaqueType, OpaqueType, OpaqueType) -> OpaqueType,
    static_set: extern "C" fn(OpaqueType, OpaqueType, OpaqueType, OpaqueType) -> c_int,
    static_invoke:
        extern "C" fn(OpaqueType, OpaqueType, OpaqueType, OpaqueTypeList, usize) -> OpaqueType,
    static_await:
        extern "C" fn(OpaqueType, OpaqueType, OpaqueType, OpaqueTypeList, usize) -> OpaqueType,
    destroy: extern "C" fn(OpaqueType, OpaqueType),
}

#[no_mangle]
extern "C" fn class_singleton_create(_klass: OpaqueType, _class_impl: OpaqueType) -> c_int {
    0
}
#[no_mangle]
extern "C" fn class_singleton_constructor(
    _klass: OpaqueType,
    _class_impl: OpaqueType,
    _name: *const char,
    _constructor: OpaqueType,
    _class_args: OpaqueTypeList,
    _size_t: usize,
) -> OpaqueType {
    0 as OpaqueType
}
#[no_mangle]
extern "C" fn class_singleton_static_set(
    _klass: OpaqueType,
    _class_impl: OpaqueType,
    _accessor: OpaqueType,
    _value: OpaqueType,
) -> c_int {
    0
}

#[no_mangle]
extern "C" fn class_singleton_static_get(
    _klass: OpaqueType,
    _class_impl: OpaqueType,
    _accessor: OpaqueType,
) -> OpaqueType {
    0 as OpaqueType
}

#[no_mangle]
extern "C" fn class_singleton_static_invoke(
    _klass: OpaqueType,
    _class_impl: OpaqueType,
    _method: OpaqueType,
    _args_p: OpaqueTypeList,
    _size: usize,
) -> OpaqueType {
    0 as OpaqueType
}

#[no_mangle]
extern "C" fn class_singleton_static_await(
    _klass: OpaqueType,
    _class_impl: OpaqueType,
    _method: OpaqueType,
    _args_p: OpaqueTypeList,
    _size: usize,
) -> OpaqueType {
    0 as OpaqueType
}

#[no_mangle]
extern "C" fn class_singleton_destroy(_klass: OpaqueType, _class_impl: OpaqueType) {}

#[no_mangle]
pub extern "C" fn class_singleton() -> *const ClassInterface {
    static SINGLETON: ClassInterface = ClassInterface {
        create: class_singleton_create,
        constructor: class_singleton_constructor,
        static_get: class_singleton_static_get,
        static_set: class_singleton_static_set,
        static_invoke: class_singleton_static_invoke,
        static_await: class_singleton_static_await,
        destroy: class_singleton_destroy,
    };

    &SINGLETON
}
pub struct ClassCreate {
    pub name: String,
    pub singleton: OpaqueType,
    pub class_impl: OpaqueType,
    pub class_info: Class,
}
pub struct ClassRegistration {
    pub ctx: OpaqueType,
    pub loader_impl: OpaqueType,
    pub class_create: ClassCreate,
}

pub fn register_class(class_registration: ClassRegistration) {
    println!("register my class!");
    let sp = unsafe { context_scope(class_registration.ctx) };

    let ClassCreate {
        name,
        class_impl,
        singleton,
        class_info,
    } = class_registration.class_create;
    let name = CString::new(name).expect("Failed to convert function name to C string");
    // dbg!(&class_info);
    // class_impl should be a Class instance. // class.init to create new instance
    // Or it can be just Host? do we really need host?
    let class = unsafe { class_create(name.as_ptr(), 0, class_impl, singleton) };

    // register ctor:
    if let Some(constructor) = class_info.constructor {
        let ctor = unsafe { constructor_create(constructor.args.len(), 0) };
        for (idx, arg) in constructor.args.iter().enumerate() {
            let name = CString::new(arg.name.clone())
                .expect("Failed to convert function parameter name to C string");
            let t = CString::new(arg.ty.to_string())
                .expect("Failed to convert function parameter type to C string");

            unsafe {
                constructor_set(
                    ctor,
                    idx,
                    name.as_ptr(),
                    loader_impl_type(class_registration.loader_impl, t.as_ptr()),
                )
            };
        }
        unsafe { class_register_constructor(class, ctor) };
    } else {
        // TODO: add default constructor
        println!("should add default constructor");
    }
    // register attrs
    for attr in class_info.attributes.iter() {
        let name =
            CString::new(attr.name.clone()).expect("Failed to convert function name to C string");
        let ty = CString::new(attr.ty.ty.to_string())
            .expect("Failed to convert function name to C string");
        let attribute = unsafe {
            attribute_create(
                class,
                name.as_ptr(),
                loader_impl_type(class_registration.loader_impl, ty.as_ptr()),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        unsafe { class_register_attribute(class, attribute) };
    }
    // we don't have static attributes in rust for now.
    // for attr in class_info.static_attributes.iter() {
    //     let static_attribute = unsafe { attribute_create(class, name, ty, null, 0, null) };
    //     unsafe { class_register_static_attribute(class, static_attribute) };
    // }
    for method in class_info.methods.iter() {
        let name =
            CString::new(method.name.clone()).expect("Failed to convert function name to C string");
        let m = unsafe {
            method_create(
                class,
                name.as_ptr(),
                method.args.len(),
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
            )
        };

        let s = unsafe { method_signature(m) };
        if let Some(ret) = &method.ret {
            let ret = CString::new(ret.ty.to_string())
                .expect("Failed to convert return type to C string");

            unsafe {
                signature_set_return(
                    s,
                    loader_impl_type(class_registration.loader_impl, ret.as_ptr()),
                );
            };
        }
        for (idx, param) in method.args.iter().enumerate() {
            let name = CString::new(param.name.clone())
                .expect("Failed to convert function parameter name to C string");
            let t = CString::new(param.ty.to_string())
                .expect("Failed to convert function parameter type to C string");

            unsafe {
                signature_set(
                    s,
                    idx,
                    name.as_ptr(),
                    loader_impl_type(class_registration.loader_impl, t.as_ptr()),
                )
            };
        }
        unsafe { class_register_method(class, m) };
    }
    for method in class_info.static_methods.iter() {
        let name =
            CString::new(method.name.clone()).expect("Failed to convert function name to C string");
        let m = unsafe {
            method_create(
                class,
                name.as_ptr(),
                method.args.len(),
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
            )
        };

        let s = unsafe { method_signature(m) };
        if let Some(ret) = &method.ret {
            let ret = CString::new(ret.ty.to_string())
                .expect("Failed to convert return type to C string");

            unsafe {
                signature_set_return(
                    s,
                    loader_impl_type(class_registration.loader_impl, ret.as_ptr()),
                );
            };
        }
        for (idx, param) in method.args.iter().enumerate() {
            let name = CString::new(param.name.clone())
                .expect("Failed to convert function parameter name to C string");
            let t = CString::new(param.ty.to_string())
                .expect("Failed to convert function parameter type to C string");

            unsafe {
                signature_set(
                    s,
                    idx,
                    name.as_ptr(),
                    loader_impl_type(class_registration.loader_impl, t.as_ptr()),
                )
            };
        }
        unsafe { class_register_static_method(class, m) };
    }
    unsafe {
        let v = value_create_class(class);
        if scope_define(sp, class_name(class), v) != 0 {
            value_type_destroy(v);
            // TODO: Should return error
        }
    };
}
