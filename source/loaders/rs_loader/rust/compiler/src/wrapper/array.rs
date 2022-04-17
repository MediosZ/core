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
            Mutability::Yes => format!("vec_p{}: *mut {}, vec_l{}: usize", self.idx, &self.ty.generic[0].name, self.idx),
            Mutability::No => format!("vec_p{}: *const {}, vec_l{}: usize", self.idx, &self.ty.generic[0].name, self.idx)
        }
    }
    fn as_ret(&self) -> String{
        format!("*mut {}", {&self.ty.generic[0].name})
    }
    fn name(&self) -> String {
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
        format!("{}{}vec{}", ref_symbol, mut_symbol, self.idx)
    }
    fn transform(&self) -> String {
        let mut_symbol = {
            match self.ty.mutability {
                Mutability::Yes => "mut ",
                Mutability::No => ""
            }
        };
        match self.ty.reference {
            Reference::Yes => {
                format!("let {}vec{}: Vec<{}> = std::slice::from_raw_parts({}, {}).to_vec();\n", 
                mut_symbol, self.idx, &self.ty.generic[0].name, format!("vec_p{}", self.idx), format!("vec_l{}", self.idx))
            },
            Reference::No => {
                format!("let {}vec{}: Vec<{}> = std::slice::from_raw_parts({}, {}).iter().cloned().collect();\n", 
                mut_symbol, self.idx, &self.ty.generic[0].name, format!("vec_p{}", self.idx), format!("vec_l{}", self.idx))
            }
        }

    }
    fn cleanup(&self) -> String {
        match self.ty.reference {
            Reference::Yes => {
                format!("std::mem::forget(vec{});\n", self.idx)
            },
            Reference::No => {
                format!("")
            }
        }
        
    }
    fn get_args_type(&self) -> std::vec::Vec<FunctionType3> {
        let pointer = FunctionType3{
            name: format!("vec_p{}", self.idx), 
            mutability: self.ty.mutability.clone(),
            reference: Reference::No,
            ty: FunctionType2::Array,
            generic: self.ty.generic.clone()
        };
        let size = FunctionType3{
            name: format!("vec_l{}", self.idx), 
            mutability: self.ty.mutability.clone(),
            reference: Reference::No,
            ty: FunctionType2::I32,
            generic: vec![]
        };
        vec![pointer, size]
    }

    fn get_ret_type(&self) -> FunctionType3 {
        FunctionType3{
            ..self.ty.clone()
        }
    }
}