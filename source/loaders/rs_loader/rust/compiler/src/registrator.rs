use std::{ffi::c_void, os::raw::c_uint};

use api::{register_function, function_singleton, FunctionCreate, FunctionInputSignature, FunctionRegisteration, Payload};

// use dlopen::raw::Library as DlopenLibrary;
use crate::file::DlopenLibrary;
use crate::{CompilerState, Function};

fn function_create(func: &Function, dlopen_library: &DlopenLibrary) -> FunctionCreate {
    let name = func.name.clone();
    let args_count = func.args.len();

    let function_ptr: unsafe fn() = unsafe { dlopen_library.instance.symbol(&name[..]) }.unwrap();
    let function_impl = Box::into_raw(Box::new(Payload{number: 123, func: Box::new(function_ptr as *mut c_void)})) as *mut c_void; // Box::into_raw(libffi_func) as *mut c_void;

    
    let function_create = FunctionCreate {
        name,
        args_count,
        function_impl,
        singleton: function_singleton as *mut c_void// 0 as c_uint as *mut c_void, // TODO: This must be a function pointer to 'function_singleton' inside the API module
    };

    function_create
}

pub fn register(state: &CompilerState, dylib: &DlopenLibrary, loader_impl: *mut c_void, ctx: *mut c_void) {
    for func in state.functions.iter() {
        let function_registration = FunctionRegisteration {
            ctx,
            loader_impl,
            function_create: function_create(func, &dylib),
            ret: match &func.ret {
                Some(ret) => Some(ret.ty.to_string().clone()),
                _ => None,
            },
            input: func
                .args
                .iter()
                .map(|param| FunctionInputSignature {
                    name: param.name.clone(),
                    t: param.ty.to_string().clone(),
                })
                .collect(),
        };

        register_function(function_registration);
    }
}
