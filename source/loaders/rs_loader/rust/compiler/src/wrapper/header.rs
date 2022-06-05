use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_double, c_float, c_int, c_long, c_short},
};
extern "C" {
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
    fn metacall_value_to_string(v: *mut c_void) -> *mut c_char;
    fn metacall_function(cfn: *const c_char) -> *mut c_void;
    fn metacall_value_create_int(i: c_int) -> *mut c_void;
    fn metacall_value_create_bool(b: c_int) -> *mut c_void;
    fn metacall_value_create_long(l: c_long) -> *mut c_void;
    fn metacall_value_create_char(st: c_char) -> *mut c_void;
    fn metacall_value_create_short(s: c_short) -> *mut c_void;
    fn metacall_value_create_float(f: c_float) -> *mut c_void;
    fn metacall_value_create_double(d: c_double) -> *mut c_void;
    fn metacall_value_create_string(st: *const c_char, ln: usize) -> *mut c_void;
    fn metacall_value_create_array(values: *const *mut c_void, size: usize) -> *mut c_void;
    fn metacall_value_create_map(tuples: *const *mut c_void, size: usize) -> *mut c_void;
}
