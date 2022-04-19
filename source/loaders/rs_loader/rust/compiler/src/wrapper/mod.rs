mod array;
mod number;
mod null;
mod map;


use crate::Function;

use super::{CompilerCallbacks, Source, FunctionType3, FunctionType2, Mutability, Reference};
use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::fmt;
trait Wrapper {
    fn as_arg(&self) -> String;
    fn transform(&self, args_ptr: &str) -> String;
    fn cleanup(&self) -> String;
    fn arg_name(&self) -> String;
    fn var_name(&self) -> String;
    fn get_args_type(&self) -> FunctionType3;
    fn get_ret_type(&self) -> FunctionType3;
    fn handle_ret(&self, ret_name: &str) -> String;
}

fn value_to_type(ty: &FunctionType2) -> String {
    match ty{
        FunctionType2::I16 | FunctionType2::U16 => "metacall_value_to_short".to_string(),
        FunctionType2::I32 | FunctionType2::U32 => "metacall_value_to_int".to_string(),
        FunctionType2::I64 | FunctionType2::U64=> "metacall_value_to_long".to_string(),
        FunctionType2::Bool=> "metacall_value_to_bool".to_string(),
        FunctionType2::Char=> "metacall_value_to_char".to_string(),
        FunctionType2::F32=> "metacall_value_to_float".to_string(),
        FunctionType2::F64=> "metacall_value_to_double".to_string(),
        _ => todo!()
    }
}

fn value_to_rust_type(ty: &FunctionType2) -> String {
    match ty{
        FunctionType2::I16 => "i16".to_string(),
        FunctionType2::I32 => "i32".to_string(),
        FunctionType2::I64 => "i64".to_string(),
        FunctionType2::U16 => "u16".to_string(),
        FunctionType2::U32 => "u32".to_string(),
        FunctionType2::U64 => "u64".to_string(),
        FunctionType2::Bool=> "bool".to_string(),
        FunctionType2::Char=> "char".to_string(),
        FunctionType2::F32=> "f32".to_string(),
        FunctionType2::F64=> "f64".to_string(),
        _ => todo!()
    }
}

fn value_create_type(ty: &FunctionType2) -> String {
    match ty{
        FunctionType2::I16 | FunctionType2::U16 => "metacall_value_create_short".to_string(),
        FunctionType2::I32 | FunctionType2::U32 => "metacall_value_create_int".to_string(),
        FunctionType2::I64 | FunctionType2::U64=> "metacall_value_create_long".to_string(),
        FunctionType2::Bool=> "metacall_value_create_bool".to_string(),
        FunctionType2::Char=> "metacall_value_create_char".to_string(),
        FunctionType2::F32=> "metacall_value_create_float".to_string(),
        FunctionType2::F64=> "metacall_value_create_double".to_string(),
        _ => todo!()
    }
}


impl fmt::Debug for dyn Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(format!("{:?}", self).as_str())?;
        Ok(())
    }
}

#[derive(Default, Debug)]
struct WrapperFunction{
    name: String,
    args: Vec<Box<dyn Wrapper>>,
    ret: Option<Box<dyn Wrapper>>
}

fn function_to_wrapper(idx: usize, typ: &FunctionType3) -> Box<dyn Wrapper> {
    // println!("{:?}", typ);
    match typ.ty {
        FunctionType2::I16 | 
        FunctionType2::I32 |
        FunctionType2::I64 | 
        FunctionType2::U16 | 
        FunctionType2::U32 | 
        FunctionType2::U64 | 
        FunctionType2::F32 | 
        FunctionType2::F64 => {
            Box::new(number::Number::new(idx, typ.clone()))
        },
        FunctionType2::Array => {
            Box::new(array::Vec::new(idx, typ.clone()))
        },
        FunctionType2::Map => {
            Box::new(map::Map::new(idx, typ.clone()))
        },
        // FunctionType2::Null => Box::new(null::Null{}),
        _ => todo!()
    }
}

impl WrapperFunction {
    fn new(func: &Function) -> Self {
        let mut result = WrapperFunction {
            name: func.name.clone(),
            args: vec![],
            ret: None
        };
        if let Some(ret) = &func.ret {
            result.ret = Some(function_to_wrapper(0, ret));
        }
        for (idx, arg) in func.args.iter().enumerate() {
            result.args.push(function_to_wrapper(idx, arg));
        }
        result
    }

    fn generate(&self) -> String {
        let mut wrapper_string = String::new();
        wrapper_string.push_str(format!("#[no_mangle]
pub unsafe fn metacall_{}(args_p: *mut *mut c_void, size: usize) -> *mut c_void {{
", self.name).as_str());
        wrapper_string.push_str("    let args_ptr = std::slice::from_raw_parts(args_p, size);\n");

        // transform
        for arg in self.args.iter() {
            wrapper_string.push_str(format!("    {}", arg.transform("args_ptr")).as_str());
        }

        // call real_func
        wrapper_string.push_str(format!("    let metacall_res = {}({});\n", 
            self.name, 
            self.args.iter().map(|arg| arg.var_name()).collect::<Vec<String>>().join(", ")
        ).as_str());

        // cleanup
        for arg in self.args.iter() {
            wrapper_string.push_str(format!("    {}", arg.cleanup()).as_str());
        }

        if let Some(ret) = &self.ret {
            wrapper_string.push_str(format!("    {} \n}}\n", ret.handle_ret("metacall_res")).as_str());
        }
        else {
            wrapper_string.push_str("    0 as *mut c_void \n}\n");
        }
        
        wrapper_string
    }
}

pub fn generate_wrapper(callbacks: CompilerCallbacks) -> std::io::Result<CompilerCallbacks>{
    // generate wrappers to a file source_wrapper.rs
    // open file source_wrapper.rs
    let mut wrapped_functions: Vec<Function> = vec![];

    let mut source_path = callbacks.source.input_path.clone();
    let source_file = source_path.file_name().expect("not a file").to_str().unwrap().to_owned();
    let _ = source_path.pop();
    source_path.push("wrapped_".to_owned() + &source_file);
    let mut wrapper_file = File::create(&source_path)?;

    wrapper_file.write_all(b"
use std::{
    ffi::{c_void, CString},
    os::raw::{c_char, c_double, c_float, c_int, c_long, c_short},
};
extern \"C\" {
    fn value_type_count(v: *mut c_void) -> c_int;
    fn metacall_value_id(v: *mut c_void) -> c_int;
    fn metacall_value_to_int(v: *mut c_void) -> c_int;
    fn metacall_value_to_bool(v: *mut c_void) -> c_int;
    fn metacall_value_to_char(v: *mut c_void) -> c_char;
    fn metacall_value_to_long(v: *mut c_void) -> c_long;
    fn metacall_value_to_short(v: *mut c_void) -> c_short;
    fn metacall_value_to_float(v: *mut c_void) -> c_float;
    fn metacall_value_to_double(v: *mut c_void) -> c_double;
    fn metacall_value_to_array(v: *mut c_void) -> *mut *mut c_void;
    fn metacall_value_to_map(v: *mut c_void) -> *mut *mut c_void;
    fn metacall_value_to_ptr(v: *mut c_void) -> *mut c_void;
    fn metacall_function(cfn: *const c_char) -> *mut c_void;
    fn metacall_value_create_int(i: c_int) -> *mut c_void;
    fn metacall_value_create_bool(b: c_int) -> *mut c_void;
    fn metacall_value_create_long(l: c_long) -> *mut c_void;
    fn metacall_value_create_char(st: c_char) -> *mut c_void;
    fn metacall_value_create_short(s: c_short) -> *mut c_void;
    fn metacall_value_create_float(f: c_float) -> *mut c_void;
    fn metacall_value_to_string(v: *mut c_void) -> *mut c_char;
    fn metacall_value_create_double(d: c_double) -> *mut c_void;
    fn metacall_value_create_string(st: *const c_char, ln: usize) -> *mut c_void;
}
")?;

    for func in callbacks.functions.iter() {
        // dbg!(func);
        let wrapper_func = WrapperFunction::new(func);
        // println!("{:?}", &wrapper_func);
        let wrapper = wrapper_func.generate();
        // println!("{}", wrapper);
        // write wrapper to file
        wrapper_file.write_all(wrapper.as_bytes())?;
        // dbg!(func);

        let mut new_function = Function {
            name: format!("metacall_{}", wrapper_func.name),
            ret: None,
            args: vec![]
        };
        if let Some(ret) = wrapper_func.ret {
            new_function.ret = Some(ret.get_ret_type());
        }
        for arg in wrapper_func.args.iter() {
            new_function.args.push(arg.get_args_type());
        }
        // println!("{:?}", new_function);
        // function need strings.
        wrapped_functions.push(new_function);
    }

    // include source_wrapper.rs
    // let mut source_file = File::open(&callbacks.source.input_path)?;
    let dst = format!("include!({:?});", callbacks.source.input_path.clone());
    // println!("{}", dst);
    wrapper_file.write_all(dst.as_bytes())?;
    // construct new callback
    
    Ok(CompilerCallbacks { source: Source::new(Source::File {
        path: source_path,
    }), functions: wrapped_functions })
    // Ok(callbacks)
}