use alloc::{string::String, vec::Vec};
use crate::word32;
#[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
use crate::word64;
use polka::Module;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeProfile { Word32, Word64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeValue {
    Word32(word32::Value),
    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
    Word64(word64::Value),
}

enum Backend {
    Word32(word32::VirtualMachine),
    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
    Word64(word64::VirtualMachine),
}

pub struct Runtime { backend: Backend }

impl Runtime {
    pub fn new(module: &Module) -> Result<Self, String> {
        if module.entry >= module.functions.len() {
            return Err(String::from("module entry is outside the function table"));
        }
        let backend = if module.flags & polka::CART_FLAG_INT32_SAFE != 0 {
            Backend::Word32(word32::VirtualMachine::new())
        } else {
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            { Backend::Word64(word64::VirtualMachine::new()) }
            #[cfg(all(target_os = "none", target_pointer_width = "32"))]
            { return Err(String::from("bare-metal word32 runtime requires INT32_SAFE module")); }
        };
        Ok(Self { backend })
    }

    pub fn profile(&self) -> RuntimeProfile {
        match self.backend {
            Backend::Word32(_) => RuntimeProfile::Word32,
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            Backend::Word64(_) => RuntimeProfile::Word64,
        }
    }

    pub fn word32_mut(&mut self) -> Option<&mut word32::VirtualMachine> {
        match &mut self.backend {
            Backend::Word32(vm) => Some(vm),
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            Backend::Word64(_) => None,
        }
    }

    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
    pub fn word64_mut(&mut self) -> Option<&mut word64::VirtualMachine> {
        match &mut self.backend { Backend::Word64(vm) => Some(vm), _ => None }
    }

    pub fn into_word32(self) -> Result<word32::VirtualMachine, Self> {
        match self.backend {
            Backend::Word32(vm) => Ok(vm),
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            backend => Err(Self { backend }),
        }
    }

    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
    pub fn into_word64(self) -> Result<word64::VirtualMachine, Self> {
        match self.backend { Backend::Word64(vm) => Ok(vm), backend => Err(Self { backend }) }
    }

    pub fn run_module(&mut self, module: &Module) -> Result<RuntimeValue, String> {
        match &mut self.backend {
            Backend::Word32(vm) => vm.run_module(module).map(RuntimeValue::Word32),
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            Backend::Word64(vm) => vm.run_module(module).map(RuntimeValue::Word64),
        }
    }

    pub fn call_export(&mut self, module: &Module, name: &str, args: &[RuntimeValue]) -> Result<RuntimeValue, String> {
        match &mut self.backend {
            Backend::Word32(vm) => {
                word32::interpreter::register::validate_module_profile(module)?;
                let values: Result<Vec<_>, String> = args.iter().map(|value| match value {
                    RuntimeValue::Word32(value) => Ok(*value),
                    #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
                    RuntimeValue::Word64(_) => Err(String::from("argument numeric profile mismatch")),
                }).collect();
                vm.call_export(module, name, &values?).map(RuntimeValue::Word32)
            }
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            Backend::Word64(vm) => {
                word64::interpreter::register::validate_module_profile(module)?;
                let values: Result<Vec<_>, String> = args.iter().map(|value| match value {
                    RuntimeValue::Word64(value) => Ok(*value), _ => Err(String::from("argument numeric profile mismatch")),
                }).collect();
                vm.call_export(module, name, &values?).map(RuntimeValue::Word64)
            }
        }
    }

    pub fn run_to_yield(&mut self, module: &Module) -> Result<(), String> {
        match &mut self.backend {
            Backend::Word32(vm) => vm.run_to_yield(module),
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            Backend::Word64(vm) => vm.run_to_yield(module),
        }
    }

    pub fn resume(&mut self, module: &Module, input: RuntimeValue) -> Result<bool, String> {
        match (&mut self.backend, input) {
            (Backend::Word32(vm), RuntimeValue::Word32(value)) => vm.resume(module, value),
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            (Backend::Word64(vm), RuntimeValue::Word64(value)) => vm.resume(module, value),
            #[cfg(not(all(target_os = "none", target_pointer_width = "32")))]
            _ => Err(String::from("resume numeric profile mismatch")),
        }
    }
}
