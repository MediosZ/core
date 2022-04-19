use std::{
    ffi::{c_void, CString},
    os::raw::{c_char, c_double, c_float, c_int, c_long, c_short},
    path::PathBuf, boxed, result, env::args,
};
use libffi::low::{CodePtr, types, ffi_cif, ffi_type, prep_cif, call, ffi_abi_FFI_DEFAULT_ABI};
use dlopen::raw::Library as DlopenLibrary;

pub struct LoaderLifecycleState {
    pub execution_paths: Vec<PathBuf>,
}
impl LoaderLifecycleState {
    pub fn new(execution_paths: Vec<PathBuf>) -> LoaderLifecycleState {
        LoaderLifecycleState { execution_paths }
    }
}
pub struct Payload{
    pub number: i32,
    pub func: Box<*mut c_void>,
}

extern "C" {
    fn loader_impl_get(loader_impl: *mut c_void) -> *mut c_void;

    fn loader_initialization_register(loader_impl: *mut c_void);

    fn loader_impl_type_define(
        loader_impl: *mut c_void,
        name: *const c_char,
        the_type: *mut c_void,
    ) -> c_int;

    fn type_create(
        type_id: c_int,
        name: *const c_char,
        type_impl: *mut c_void,
        singleton: *mut c_void,
    ) -> *mut c_void;

    fn type_name(t: *mut c_void) -> *const c_char;
    
    fn value_type_id(t: *mut c_void) -> i32;

    fn value_to_array(v: *mut c_void) -> *mut *mut c_void;

    fn function_create(
        name: *const c_char,
        args_count: usize,
        function_impl: *mut c_void,
        singleton: *mut c_void,
    ) -> *mut c_void;

    fn signature_set(signature: *mut c_void, index: usize, name: *const c_char, t: *mut c_void);

    fn context_scope(ctx: *mut c_void) -> *mut c_void;

    fn function_name(function: *mut c_void) -> *mut c_char;

    fn function_signature(function: *mut c_void) -> *mut c_void;

    fn value_create_function(function: *mut c_void) -> *mut c_void;

    fn value_type_destroy(v: *mut c_void);

    fn signature_set_return(signature: *mut c_void, t: *mut c_void);

    fn loader_impl_type(loader_impl: *mut c_void, name: *const c_char) -> *mut c_void;

    fn scope_define(scope: *mut c_void, key: *mut c_char, value: *mut c_void) -> c_int;
    pub fn value_type_count(v: *mut c_void) -> c_int;
    pub fn metacall_value_id(v: *mut c_void) -> c_int;
    pub fn metacall_value_to_int(v: *mut c_void) -> c_int;
    pub fn metacall_value_to_bool(v: *mut c_void) -> c_int;
    pub fn metacall_value_to_char(v: *mut c_void) -> c_char;
    pub fn metacall_value_to_long(v: *mut c_void) -> c_long;
    pub fn metacall_value_to_short(v: *mut c_void) -> c_short;
    pub fn metacall_value_to_float(v: *mut c_void) -> c_float;
    pub fn metacall_value_to_double(v: *mut c_void) -> c_double;
    pub fn metacall_value_to_array(v: *mut c_void) -> *mut *mut c_void;
    pub fn metacall_value_to_map(v: *mut c_void) -> *mut *mut c_void;
    pub fn metacall_value_to_ptr(v: *mut c_void) -> *mut c_void;
    pub fn metacall_function(cfn: *const c_char) -> *mut c_void;
    pub fn metacall_value_create_int(i: c_int) -> *mut c_void;
    pub fn metacall_value_create_bool(b: c_int) -> *mut c_void;
    pub fn metacall_value_create_long(l: c_long) -> *mut c_void;
    pub fn metacall_value_create_char(st: c_char) -> *mut c_void;
    pub fn metacall_value_create_short(s: c_short) -> *mut c_void;
    pub fn metacall_value_create_float(f: c_float) -> *mut c_void;
    pub fn metacall_value_to_string(v: *mut c_void) -> *mut c_char;
    pub fn metacall_value_create_double(d: c_double) -> *mut c_void;
    pub fn metacall_value_create_string(st: *const c_char, ln: usize) -> *mut c_void;
}


#[repr(C)]
pub struct FunctionInterface {
    create: extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    invoke: extern "C" fn(*mut c_void, *mut c_void, *mut *mut c_void, usize) -> *mut c_void,
    r#await: extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut *mut c_void,
        usize,
        extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
        extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
        *mut c_void,
    ) -> *mut c_void,
    destroy: extern "C" fn(*mut c_void, *mut c_void),
}

#[no_mangle]
extern "C" fn function_singleton_create(_func: *mut c_void, func_impl: *mut c_void) -> c_int {
    0
}

#[no_mangle]
extern "C" fn function_singleton_invoke(
    _func: *mut c_void,
    func_impl: *mut c_void,
    args_p: *mut *mut c_void,
    size: usize,
) -> *mut c_void {
    unsafe {
        let payload = Box::from_raw(func_impl as *mut Payload);
        let func: fn(*mut *mut c_void, usize) -> *mut c_void = std::mem::transmute_copy(&(*payload.func));
        let result =  func(args_p, size);
        std::mem::forget(payload);
        result
    }
}

#[no_mangle]
extern "C" fn function_singleton_await(
    _func: *mut c_void,
    _func_impl: *mut c_void,
    _args: *mut *mut c_void,
    _size: usize,
    _resolve: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    _reject: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    _data: *mut c_void,
) -> *mut c_void {
    0 as *mut c_void
}

#[no_mangle]
extern "C" fn function_singleton_destroy(_func: *mut c_void, func_impl: *mut c_void) {
    unsafe {
        let payload = Box::from_raw(func_impl as *mut Payload);
        drop(payload);
    }
    // Here we have to free the memory of this: https://github.com/metacall/core/blob/44564a0a183a121eec4a55bcb433d52a308e5e9d/source/loaders/rs_loader/rust/compiler/src/registrator.rs#L19
}

#[no_mangle]
pub extern "C" fn function_singleton() -> *const FunctionInterface {
    static SINGLETON: FunctionInterface = FunctionInterface {
        create: function_singleton_create,
        invoke: function_singleton_invoke,
        r#await: function_singleton_await,
        destroy: function_singleton_destroy,
    };

    &SINGLETON
}

pub fn get_loader_lifecycle_state(loader_impl: *mut c_void) -> *mut LoaderLifecycleState {
    let loader_lifecycle_state =
        unsafe { loader_impl_get(loader_impl) } as *mut LoaderLifecycleState;

    loader_lifecycle_state
}

pub fn loader_lifecycle_register(loader_impl: *mut c_void) {
    unsafe { loader_initialization_register(loader_impl) };
}

pub enum PrimitiveMetacallProtocolTypes {
    Bool = 0,
    Char = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    String = 7,
    Buffer = 8,
    Array = 9,
    Map = 10,
    Pointer = 11,
    Future = 12,
    Function = 13,
    Null = 14,
    Class = 15,
    Object = 16,
}

pub fn define_type(
    loader_impl: *mut c_void,
    name: &str,
    type_id: PrimitiveMetacallProtocolTypes,
    type_impl: *mut c_void,
    singleton: *mut c_void,
) {
    let name = CString::new(name).expect("Failed to convert type name to C string");
    let type_id = type_id as c_int;

    unsafe {
        let t = type_create(type_id, name.as_ptr(), type_impl, singleton);

        loader_impl_type_define(loader_impl, type_name(t), t)
    };
}

pub struct FunctionCreate {
    pub name: String,
    pub args_count: usize,
    pub singleton: *mut c_void,
    pub function_impl: *mut c_void,
}
pub struct FunctionInputSignature {
    pub name: String,
    pub t: String,
}
pub struct FunctionRegisteration {
    pub ctx: *mut c_void,
    pub loader_impl: *mut c_void,
    pub function_create: FunctionCreate,
    pub ret: Option<String>,
    pub input: Vec<FunctionInputSignature>,
}

pub fn register_function(function_registeration: FunctionRegisteration) {
    let sp = unsafe { context_scope(function_registeration.ctx) };

    let FunctionCreate {
        name,
        args_count,
        function_impl,
        singleton,
    } = function_registeration.function_create;
    let name = CString::new(name).expect("Failed to convert function name to C string");
    let f = unsafe { function_create(name.as_ptr(), args_count, function_impl, singleton) };

    let s = unsafe { function_signature(f) };

    if let Some(ret) = function_registeration.ret {
        let ret = CString::new(ret).expect("Failed to convert return type to C string");

        unsafe {
            signature_set_return(
                s,
                loader_impl_type(function_registeration.loader_impl, ret.as_ptr()),
            );
        };
    }

    for (index, param) in function_registeration.input.iter().enumerate() {
        let name = CString::new(param.name.clone())
            .expect("Failed to convert function parameter name to C string");
        let t = CString::new(param.t.clone())
            .expect("Failed to convert function parameter type to C string");

        unsafe {
            signature_set(
                s,
                index,
                name.as_ptr(),
                loader_impl_type(function_registeration.loader_impl, t.as_ptr()),
            )
        };
    }

    unsafe {
        let v = value_create_function(f);
        if scope_define(sp, function_name(f), v) != 0 {
            value_type_destroy(v);
            // TODO: Should return error
        }
    };
}
