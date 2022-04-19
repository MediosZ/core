use crate::{FunctionType3,FunctionType2, Reference, Mutability};

use super::{Wrapper, value_to_type, value_create_type, value_to_rust_type};

#[derive(Debug)]
pub struct Vec{
    idx: usize,
    ty: FunctionType3,
}

impl Vec {
    pub fn new(idx: usize, ty: FunctionType3) -> Self {
        Self{
            idx, 
            ty
        }
    }
}

impl Wrapper for Vec{
    fn as_arg(&self) -> String {
        match self.ty.mutability {
            Mutability::Yes => format!("{}: *mut c_void", self.arg_name()),
            Mutability::No => format!("{}: *mut c_void", self.arg_name())
        }
    }

    fn arg_name(&self) -> String {
        format!("vec{}", self.idx)
    }
    fn var_name(&self) -> String {
        let mut_symbol = {
            match self.ty.mutability {
                Mutability::Yes => "mut ",
                Mutability::No => ""
            }
        };
        let ref_symbol = {
            match self.ty.reference {
                Reference::Yes => "&",
                Reference::No => ""
            }
        };

        
        format!("{}{}r_vec{}", ref_symbol, mut_symbol, self.idx)
    }
    fn transform(&self, args_ptr: &str) -> String {
        let arr_ptr = format!("{}[{}]", args_ptr, self.idx);
        let idx = self.idx;
        let mut_symbol = {
            match self.ty.mutability {
                Mutability::Yes => "mut ",
                Mutability::No => ""
            }
        };
        match self.ty.reference {
            Reference::Yes => {
                format!("let arr{idx} = metacall_value_to_array({arr_ptr});
    let count{idx} = value_type_count({arr_ptr});
    let {mut_symbol}r_vec{idx} = 
        std::slice::from_raw_parts(arr{idx}, count{idx} as usize)
        .iter()
        .map(|p| {}(*p))
        .collect::<Vec<{}>>();\n", 
            value_to_type(&self.ty.generic[0].ty), value_to_rust_type(&self.ty.generic[0].ty))
            },
            Reference::No => {
                format!("let arr{idx} = metacall_value_to_array({arr_ptr});
    let count{idx} = value_type_count({arr_ptr});
    let {mut_symbol}r_vec{idx}= 
        std::slice::from_raw_parts(arr{idx}, count{idx} as usize)
        .iter()
        .map(|p| {}(*p))
        .collect::<Vec<{}>>()
        .clone();\n", 
            value_to_type(&self.ty.generic[0].ty), value_to_rust_type(&self.ty.generic[0].ty))
            },
        }
    }
    fn cleanup(&self) -> String {
        match self.ty.reference {
            Reference::Yes => {
                format!("std::mem::forget(r_vec{});\n", self.idx)
            },
            Reference::No => {
                format!("")
            }
        }
    }

    fn handle_ret(&self, ret_name: &str) -> String {
        format!("metacall_value_create_int({})", ret_name)
    }
    fn get_args_type(&self) -> FunctionType3 {
        FunctionType3{
            name: self.arg_name(), 
            mutability: self.ty.mutability.clone(),
            reference: Reference::No,
            ty: FunctionType2::Array,
            generic: self.ty.generic.clone()
        }
    }

    fn get_ret_type(&self) -> FunctionType3 {
        FunctionType3{
            ..self.ty.clone()
        }
    }
}

/*
#[no_mangle]
fn metacall_vec(vec0: *mut c_void) -> *mut c_void {
    let arr0 = metacall_value_to_array(vec0);
    let count0 = value_type_count(vec0);
    let r_vec0: Vec<i32> = 
        std::slice::from_raw_parts(arr0, count0 as usize)
        .iter()
        .map(|p| metacall_value_to_int(*p))
        .collect();
    let metacall_res = sum(r_vec0, );
    metacall_res as *mut c_void
} */