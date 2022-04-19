use crate::{FunctionType3, FunctionType2};

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
        format!("{}: *mut c_void, ", self.arg_name())
    }
    fn arg_name(&self) -> String {
        format!("num{}", self.idx)
    }
    fn var_name(&self) -> String {
        format!("var_num{}", self.idx)
    }
    fn transform(&self, args_ptr: &str) -> String {
        format!("let {} = metacall_value_to_int({}[{}]);\n", self.var_name(), args_ptr, self.idx)
    }
    fn cleanup(&self) -> String {
        format!("\n")
    }

    fn handle_ret(&self, ret_name: &str) -> String {
        format!("metacall_value_create_int({})", ret_name)
    }
    fn get_args_type(&self) -> Vec<FunctionType3> {
        vec![FunctionType3{
            name: self.arg_name(),
            ty: FunctionType2::Ptr,
            ..self.ty.clone()
        }]
    }

    fn get_ret_type(&self) -> FunctionType3 {
        FunctionType3{
            ty: FunctionType2::Ptr,
            ..self.ty.clone()
        }
    }
}

/*
#[no_mangle]
fn metacall_number(num0: *mut c_void) -> *mut c_void {
    let r_num0 = metacall_value_to_int(num0);
    let metacall_res = number(r_num0, );
    metacall_res as *mut c_void
} */