use crate::{FunctionType3,FunctionType2, Reference, Mutability};

use super::Wrapper;

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
            Mutability::Yes => format!("vec{}: *mut c_void", self.idx),
            Mutability::No => format!("vec{}: *mut c_void", self.idx)
        }
    }
    // fn as_ret(&self) -> String{
    //     format!("*mut {}", {&self.ty.generic[0].name})
    // }

    fn arg_name(&self) -> String {
        format!("num{}", self.idx)
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
        let mut_symbol = {
            match self.ty.mutability {
                Mutability::Yes => "mut ",
                Mutability::No => ""
            }
        };
        match self.ty.reference {
            Reference::Yes => {
                format!("let arr{} = metacall_value_to_array({});
    let count{} = value_type_count({});
    let {}r_vec{} = 
        std::slice::from_raw_parts(arr{}, count{} as usize)
        .iter()
        .map(|p| metacall_value_to_int(*p))
        .collect::<Vec<i32>>();\n", 
            self.idx, arr_ptr, self.idx, arr_ptr, mut_symbol, self.idx, self.idx, self.idx)
            },
            Reference::No => {
                format!("let arr{} = metacall_value_to_array({});
    let count{} = value_type_count({});
    let {}r_vec{}= 
        std::slice::from_raw_parts(arr{}, count{} as usize)
        .iter()
        .map(|p| metacall_value_to_int(*p))
        .collect::<Vec<i32>>()
        .clone();\n", 
            self.idx, arr_ptr, self.idx, arr_ptr, mut_symbol, self.idx, self.idx, self.idx)
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
    fn get_args_type(&self) -> std::vec::Vec<FunctionType3> {
        let pointer = FunctionType3{
            name: format!("vec{}", self.idx), 
            mutability: self.ty.mutability.clone(),
            reference: Reference::No,
            ty: FunctionType2::Array,
            generic: self.ty.generic.clone()
        };
        vec![pointer]
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