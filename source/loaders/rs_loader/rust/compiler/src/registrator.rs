use std::{ffi::c_void, os::raw::c_uint};

use api::{register_function, function_singleton, FunctionCreate, FunctionInputSignature, FunctionRegisteration};

use dlopen::raw::Library as DlopenLibrary;

use libffi::low::CodePtr;
use libffi::low::*;

use crate::{CompilerState, Function};

fn function_create(func: &Function, dlopen_library: &DlopenLibrary) -> FunctionCreate {
    let name = func.name.clone();
    let args_count = func.args.len();

    let function_ptr: unsafe fn() = unsafe { dlopen_library.symbol(&name[..]) }.unwrap();

    let libffi_func = Box::new(CodePtr::from_ptr(function_ptr as *const c_void));

    println!("{:?}", *libffi_func);
    let function_impl = Box::into_raw(libffi_func) as *mut c_void;

    unsafe {
        //prepare rust args
        let mut args: Vec<*mut ffi_type> = vec![ &mut types::sint32,
                                                &mut types::sint32 ];
        let mut cif: ffi_cif = Default::default();
        prep_cif(&mut cif, ffi_abi_FFI_DEFAULT_ABI, args_count,
                &mut types::sint32, args.as_mut_ptr()).unwrap();
        // let libffi_func: Box<CodePtr> = std::mem::transmute(func_impl);
        let boxed_func_impl = Box::from_raw(function_impl as *mut CodePtr);
        println!("{:?}", *boxed_func_impl);
        println!("before call");
        let result: i32 = call(&mut cif, 
            *boxed_func_impl, 
            vec![ &mut 5i32 as *mut _ as *mut c_void, &mut 6i32 as *mut _ as *mut c_void ].as_mut_ptr()
        );

        println!("after call");
        println!("get result: {}", result);
        std::mem::forget(boxed_func_impl);
    };
    let function_create = FunctionCreate {
        name,
        args_count,
        function_impl,
        singleton: function_singleton as *mut c_void// 0 as c_uint as *mut c_void, // TODO: This must be a function pointer to 'function_singleton' inside the API module
    };

    function_create
}

pub fn register(state: &CompilerState, loader_impl: *mut c_void, ctx: *mut c_void) {
    let dlopen_library = DlopenLibrary::open(state.output.clone()).unwrap();

    for func in state.functions.iter() {
        let function_registration = FunctionRegisteration {
            ctx,
            loader_impl,
            function_create: function_create(func, &dlopen_library),
            ret: match &func.ret {
                Some(ret) => Some(ret.name.clone()),
                _ => None,
            },
            input: func
                .args
                .iter()
                .map(|param| FunctionInputSignature {
                    name: param.name.clone(),
                    t: param.t.name.clone(),
                })
                .collect(),
        };

        register_function(function_registration);
    }
}
