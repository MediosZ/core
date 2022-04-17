use crate::FunctionType3;

use super::Wrapper;

#[derive(Debug, Clone)]
pub struct Number{
    idx: usize,
    ty: FunctionType3,
}

impl Number {
    pub fn new(idx: usize, ty: FunctionType3) -> Self {
        Self{
            idx, 
            ty
        }
    }
}

impl Wrapper for Number {
    fn as_arg(&self) -> String {
        format!("num{}: {}, ", self.idx, self.ty.name)
    }
    fn as_ret(&self) -> String{
        format!("{}", self.ty.name)
    }
    fn name(&self) -> String {
        format!("num{}", self.idx)
    }
    fn transform(&self) -> String {
        format!("\n")
    }
    fn cleanup(&self) -> String {
        format!("\n")
    }
    fn get_args_type(&self) -> Vec<FunctionType3> {
        vec![FunctionType3{
            name: self.name(),
            ..self.ty.clone()
        }]
    }

    fn get_ret_type(&self) -> FunctionType3 {
        FunctionType3{
            ..self.ty.clone()
        }
    }
}

/*
#[no_mangle]
fn metacall_add_vec_inner(vec_p0: *mut i32, vec_l0: usize) -> i32 {
    let vec0: Vec<i32> = std::slice::from_raw_parts(vec_p0, vec_l0).to_vec();
    let metacall_res = add_vec_inner(vec0, );
    std::mem::forget(vec0);
    metacall_res 
} */