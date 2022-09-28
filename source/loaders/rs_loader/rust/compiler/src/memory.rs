use crate::{compile, CompilerState, RegistrationError, Source};

use std::{ffi::c_void, fs};

use crate::{registrator, DlopenLibrary};

#[derive(Debug)]
pub struct MemoryRegistration {
    pub name: String,
    pub state: CompilerState,
    pub dlopen: Option<DlopenLibrary>,
}
impl MemoryRegistration {
    pub fn new(name: String, code: String) -> Result<MemoryRegistration, RegistrationError> {
        let state = match compile(Source::new(Source::Memory {
            name: name.clone(),
            code,
        })) {
            Ok(state) => state,
            Err(error) => {
                return Err(RegistrationError::CompilationError(String::from(format!(
                    "{}\n{}\n{}",
                    error.err, error.errors, error.diagnostics
                ))))
            }
        };
        let dlopen = match DlopenLibrary::new(&state.output) {
            Ok(instance) => instance,
            Err(error) => return Err(RegistrationError::DlopenError(error)),
        };
        // cleanup temp dir
        let mut destination = state.output.clone();
        destination.pop();
        std::fs::remove_dir_all(destination).expect("Unable to cleanup tempdir");

        Ok(MemoryRegistration {
            name,
            state,
            dlopen: Some(dlopen),
        })
    }

    pub fn discover(&self, loader_impl: *mut c_void, ctx: *mut c_void) -> Result<(), String> {
        match &self.dlopen {
            Some(dl) => {
                registrator::register(&self.state, &dl, loader_impl, ctx);
                Ok(())
            }
            None => Err(String::from("The dlopen_lib is None")),
        }
    }
}
