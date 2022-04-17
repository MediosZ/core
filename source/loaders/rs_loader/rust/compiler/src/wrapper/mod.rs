mod array;
mod number;
mod null;


use crate::Function;

use super::{CompilerCallbacks, Source, FunctionType3, FunctionType2};
use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::fmt;
trait Wrapper {
    fn as_arg(&self) -> String;
    fn as_ret(&self) -> String;
    fn transform(&self) -> String;
    fn cleanup(&self) -> String;
    fn name(&self) -> String;
    fn get_args_type(&self) -> Vec<FunctionType3>;
    fn get_ret_type(&self) -> FunctionType3;
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
        // headers
        wrapper_string.push_str("#[no_mangle]\npub unsafe fn ");
        // func name
        wrapper_string.push_str(format!("metacall_{}", self.name).as_str());
        // args 
        wrapper_string.push_str("(");
        for arg in self.args.iter() {
            wrapper_string.push_str(arg.as_arg().as_str());
        }
        wrapper_string.push_str(")");
        // ret
        if let Some(ret) = &self.ret {
            wrapper_string.push_str(" -> ");
            wrapper_string.push_str(ret.as_ret().as_str());
        }
        wrapper_string.push_str(" {\n");
        // transform
        for arg in self.args.iter() {
            wrapper_string.push_str(format!("    {}", arg.transform()).as_str());
        }
        // call real_func
        wrapper_string.push_str(format!("    let metacall_res = {}(", self.name).as_str());
        for arg in self.args.iter() {
            wrapper_string.push_str(format!("{}, ", arg.name()).as_str());
        }
        wrapper_string.push_str(");\n");
        // cleanup
        for arg in self.args.iter() {
            wrapper_string.push_str(format!("    {}", arg.cleanup()).as_str());
        }
        wrapper_string.push_str("    metacall_res \n}\n");
        wrapper_string
    }
}

// fn type_to_wrapper(ty: &FunctionType) -> Box<dyn Wrapper> {
//     match ty.name.as_str() {
//         "i16" | "i32" | "i64" |
//         "u16" | "u32" | "u64" |
//         "f32" | "f64" => Box::new(number::Number{}),
//         "vec" => Box::new(array::Vec{}),
//         _ => Box::new(null::Null{})
//     }
// }

pub fn generate_wrapper(callbacks: CompilerCallbacks) -> std::io::Result<CompilerCallbacks>{
    println!("generate wrapper for ...");
    // generate wrappers to a file source_wrapper.rs
    // open file source_wrapper.rs
    let mut wrapped_functions: Vec<Function> = vec![];

    let mut source_path = callbacks.source.input_path.clone();
    let source_file = source_path.file_name().expect("not a file").to_str().unwrap().to_owned();
    let _ = source_path.pop();
    source_path.push("wrapped_".to_owned() + &source_file);
    let mut wrapper_file = File::create(&source_path)?;

    for func in callbacks.functions.iter() {
        dbg!(func);
        let wrapper_func = WrapperFunction::new(func);
        // println!("{:?}", &wrapper_func);
        let wrapper = wrapper_func.generate();
        println!("{}", wrapper);
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
            new_function.args.extend(arg.get_args_type());
        }
        // function need strings.
        wrapped_functions.push(new_function);
    }
    // include source_wrapper.rs
    // let mut source_file = File::open(&callbacks.source.input_path)?;
    let dst = format!("include!({:?});", callbacks.source.input_path.clone());
    // println!("{}", dst);
    // source_file.write_all(dst.as_bytes())?;
    wrapper_file.write_all(dst.as_bytes())?;
    // construct new callback
    
    Ok(CompilerCallbacks { source: Source::new(Source::File {
        path: source_path,
    }), functions: wrapped_functions })
    // Ok(callbacks)
}