/* EVMC: Ethereum Client-VM Connector API.
 * Copyright 2019 The EVMC Authors.
 * Licensed under the Apache License, Version 2.0.
 */

pub extern crate evmc_sys;
pub use evmc_sys as ffi;
pub use paste::expr;
pub use paste::item;

// TODO: Add helpers for host interface
// TODO: Add convenient helpers for evmc_execute
// TODO: Add a derive macro here for creating evmc_create

/// EVMC result structure.
pub struct ExecutionResult {
    status_code: ffi::evmc_status_code,
    gas_left: i64,
    output: Option<Vec<u8>>,
    create_address: ffi::evmc_address,
}

pub trait EvmcVM {
    fn init() -> Self;
    fn execute(&self, code: &[u8] /*context goes here*/) -> ExecutionResult;
}

impl ExecutionResult {
    pub fn new(
        _status_code: ffi::evmc_status_code,
        _gas_left: i64,
        _output: Option<Vec<u8>>,
        _create_address: Option<ffi::evmc_address>,
    ) -> Self {
        ExecutionResult {
            status_code: _status_code,
            gas_left: _gas_left,
            output: _output,
            create_address: {
                if let Some(_create_address) = _create_address {
                    _create_address
                } else {
                    ffi::evmc_address { bytes: [0u8; 20] }
                }
            },
        }
    }

    pub fn get_status_code(&self) -> ffi::evmc_status_code {
        self.status_code
    }

    pub fn get_gas_left(&self) -> i64 {
        self.gas_left
    }

    pub fn get_output(&self) -> Option<&Vec<u8>> {
        self.output.as_ref()
    }

    pub fn get_create_address(&self) -> Option<&ffi::evmc_address> {
        // Only return Some if the address is valid (e.g. the status is EVMC_SUCCESS)
        if self.status_code == ffi::evmc_status_code::EVMC_SUCCESS {
            Some(&self.create_address)
        } else {
            None
        }
    }
}

impl From<ffi::evmc_result> for ExecutionResult {
    fn from(result: ffi::evmc_result) -> Self {
        let ret = ExecutionResult {
            status_code: result.status_code,
            gas_left: result.gas_left,
            output: if !result.output_data.is_null() {
                // Pre-allocate a vector.
                let mut buf: Vec<u8> = Vec::with_capacity(result.output_size);

                unsafe {
                    // Set the len of the vec manually.
                    buf.set_len(result.output_size);
                    // Copy from the C struct's buffer to the vec's buffer.
                    std::ptr::copy(result.output_data, buf.as_mut_ptr(), result.output_size);
                }

                Some(buf)
            } else {
                None
            },
            create_address: result.create_address,
        };

        // Release allocated ffi struct.
        if result.release.is_some() {
            unsafe {
                result.release.unwrap()(&result as *const ffi::evmc_result);
            }
        }

        ret
    }
}

fn allocate_output_data(output: Option<Vec<u8>>) -> (*const u8, usize) {
    if let Some(buf) = output {
        let buf_len = buf.len();

        // Manually allocate heap memory for the new home of the output buffer.
        let memlayout = std::alloc::Layout::from_size_align(buf_len, 1).expect("Bad layout");
        let new_buf = unsafe { std::alloc::alloc(memlayout) };
        unsafe {
            // Copy the data into the allocated buffer.
            std::ptr::copy(buf.as_ptr(), new_buf, buf_len);
        }

        (new_buf as *const u8, buf_len)
    } else {
        (std::ptr::null(), 0)
    }
}

unsafe fn deallocate_output_data(ptr: *const u8, size: usize) {
    if !ptr.is_null() {
        let buf_layout = std::alloc::Layout::from_size_align(size, 1).expect("Bad layout");
        std::alloc::dealloc(ptr as *mut u8, buf_layout);
    }
}

/// Returns a pointer to a heap-allocated evmc_result.
impl Into<*const ffi::evmc_result> for ExecutionResult {
    fn into(self) -> *const ffi::evmc_result {
        let (buffer, len) = allocate_output_data(self.output);
        Box::into_raw(Box::new(ffi::evmc_result {
            status_code: self.status_code,
            gas_left: self.gas_left,
            output_data: buffer,
            output_size: len,
            release: Some(release_heap_result),
            create_address: self.create_address,
            padding: [0u8; 4],
        }))
    }
}

/// Callback to pass across FFI, de-allocating the optional output_data.
extern "C" fn release_heap_result(result: *const ffi::evmc_result) {
    unsafe {
        let tmp = Box::from_raw(result as *mut ffi::evmc_result);
        deallocate_output_data(tmp.output_data, tmp.output_size);
    }
}

/// Returns a pointer to a stack-allocated evmc_result.
impl Into<ffi::evmc_result> for ExecutionResult {
    fn into(self) -> ffi::evmc_result {
        let (buffer, len) = allocate_output_data(self.output);
        ffi::evmc_result {
            status_code: self.status_code,
            gas_left: self.gas_left,
            output_data: buffer,
            output_size: len,
            release: Some(release_stack_result),
            create_address: self.create_address,
            padding: [0u8; 4],
        }
    }
}

/// Callback to pass across FFI, de-allocating the optional output_data.
extern "C" fn release_stack_result(result: *const ffi::evmc_result) {
    unsafe {
        let tmp = *result;
        deallocate_output_data(tmp.output_data, tmp.output_size);
    }
}

pub mod macros {
    #[macro_export]
    macro_rules! evmc_create_vm {
        ($__vm:ident, $__version:expr) => {
            item! {
                static [<$__vm _NAME>]: &'static str = stringify!($__vm);
                static [<$__vm _VERSION>]: &'static str = $__version;
            }

            item! {
                #[derive(Clone)]
                #[repr(C)]
                struct [<$__vm Instance>] {
                    inner: ffi::evmc_instance,
                    vm: $__vm,
                }
            }

            item! {
                impl [<$__vm Instance>] {
                    pub fn new() -> Self {
                        //$__vm must implement EvmcVM
                        [<$__vm Instance>] {
                            inner: ffi::evmc_instance {
                                abi_version: ffi::EVMC_ABI_VERSION as i32,
                                destroy: expr! { Some([<$__vm _destroy>]) },
                                execute: expr! { Some([<$__vm _execute>]) },
                                get_capabilities: expr! { Some([<$__vm _get_capabilities>]) },
                                set_option: None,
                                set_tracer: None,
                                name: {
                                    let c_str = expr! { std::ffi::CString::new([<$__vm _NAME>]).expect("Failed to build EVMC name string") };
                                    c_str.into_raw() as *const i8
                                },
                                version: {
                                    let c_str = expr! { std::ffi::CString::new([<$__vm _VERSION>]).expect("Failed to build EVMC version string") };
                                    c_str.into_raw() as *const i8
                                },
                            },
                            vm: $__vm::init(),
                        }
                    }

                    pub fn get_vm(&self) -> &$__vm {
                        &self.vm
                    }

                    pub fn get_inner(&self) -> &ffi::evmc_instance {
                        &self.inner
                    }

                    pub fn into_inner_raw(self) -> *mut ffi::evmc_instance {
                        Box::into_raw(Box::new(self)) as *mut ffi::evmc_instance
                    }

                    // Assumes the pointer is casted from another instance of Self. otherwise UB
                    pub unsafe fn coerce_from_raw(raw: *mut ffi::evmc_instance) -> Self {
                        let borrowed = (raw as *mut [<$__vm Instance>]).as_ref();
                        if let Some(instance) = borrowed {
                            let ret = instance.clone();
                            // deallocate the old heap-allocated instance.
                            Box::from_raw(raw);
                            ret
                        } else {
                            panic!();
                        }
                    }
                }
            }

            item! {
                extern "C" fn [<$__vm _execute>](
                    instance: *mut ffi::evmc_instance,
                    context: *mut ffi::evmc_context,
                    rev: ffi::evmc_revision,
                    msg: *const ffi::evmc_message,
                    code: *const u8,
                    code_size: usize,
                ) -> ffi::evmc_result {
                    assert!(code_size < std::isize::MAX as usize);
                    assert!(!code.is_null());
                    let code_ref: &[u8] = unsafe {
                        std::slice::from_raw_parts(code, code_size)
                    };

                    assert!(!msg.is_null());
                    assert!(!context.is_null());
                    assert!(!instance.is_null());
                    /*
                    let host = unsafe {
                        InterfaceManager::new(&rev,
                            &*msg,
                            &mut *context,
                            &mut *instance)
                    };
                    */

                    let instance = unsafe { [<$__vm Instance>]::coerce_from_raw(instance) };
                    let result: ExecutionResult = instance.get_vm().execute(code_ref /*interface goes here*/);
                                    result.into()
                    //ffi::evmc_result {
                    //    create_address: ffi::evmc_address { bytes: [0u8; 20] },
                    //    gas_left: 0,
                    //    output_data: 0 as *const u8,
                    //    output_size: 0,
                    //    release: None,
                    //    status_code: ffi::evmc_status_code::EVMC_FAILURE,
                    //    padding: [0u8; 4],
                    //}
                }
            }

            item! {
                extern "C" fn [<$__vm _get_capabilities>](instance: *mut ffi::evmc_instance) -> ffi::evmc_capabilities_flagset {
                    ffi::evmc_capabilities::EVMC_CAPABILITY_EVM1 as u32
                }
            }

            item! {
                extern "C" fn [<$__vm _destroy>](instance: *mut ffi::evmc_instance) {
                    // The EVMC specification ensures instance cannot be null.
                    // Cast to the enclosing struct so that the extra data gets deallocated too.
                    let todrop = instance as *mut [<$__vm Instance>];
                    drop(unsafe { Box::from_raw(todrop) })
                }
            }

            item! {
                #[no_mangle]
                extern "C" fn [<evmc_create_ $__vm>]() -> *const ffi::evmc_instance {
                    expr! { [<$__vm Instance>]::new().into_inner_raw() }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_result() {
        let r = ExecutionResult::new(
            ffi::evmc_status_code::EVMC_FAILURE,
            420,
            None,
            Some(ffi::evmc_address { bytes: [0u8; 20] }),
        );

        assert!(r.get_status_code() == ffi::evmc_status_code::EVMC_FAILURE);
        assert!(r.get_gas_left() == 420);
        assert!(r.get_output().is_none());

        // Ensure that an address is not returned if it is not valid, per status code.
        assert!(r.get_create_address().is_none());
    }

    #[test]
    fn from_ffi() {
        let f = ffi::evmc_result {
            status_code: ffi::evmc_status_code::EVMC_SUCCESS,
            gas_left: 1337,
            output_data: Box::into_raw(Box::new([0xde, 0xad, 0xbe, 0xef])) as *const u8,
            output_size: 4,
            release: None,
            create_address: ffi::evmc_address { bytes: [0u8; 20] },
            padding: [0u8; 4],
        };

        let r: ExecutionResult = f.into();

        assert!(r.get_status_code() == ffi::evmc_status_code::EVMC_SUCCESS);
        assert!(r.get_gas_left() == 1337);
        assert!(r.get_output().is_some());
        assert!(r.get_output().unwrap().len() == 4);
        assert!(r.get_create_address().is_some());
    }

    #[test]
    fn into_heap_ffi() {
        let r = ExecutionResult::new(
            ffi::evmc_status_code::EVMC_FAILURE,
            420,
            Some(vec![0xc0, 0xff, 0xee, 0x71, 0x75]),
            Some(ffi::evmc_address { bytes: [0u8; 20] }),
        );

        let f: *const ffi::evmc_result = r.into();
        assert!(!f.is_null());
        unsafe {
            assert!((*f).status_code == ffi::evmc_status_code::EVMC_FAILURE);
            assert!((*f).gas_left == 420);
            assert!(!(*f).output_data.is_null());
            assert!((*f).output_size == 5);
            assert!(
                std::slice::from_raw_parts((*f).output_data, 5) as &[u8]
                    == &[0xc0, 0xff, 0xee, 0x71, 0x75]
            );
            assert!((*f).create_address.bytes == [0u8; 20]);
            if (*f).release.is_some() {
                (*f).release.unwrap()(f);
            }
        }
    }

    #[test]
    fn into_heap_ffi_empty_data() {
        let r = ExecutionResult::new(
            ffi::evmc_status_code::EVMC_FAILURE,
            420,
            None,
            Some(ffi::evmc_address { bytes: [0u8; 20] }),
        );

        let f: *const ffi::evmc_result = r.into();
        assert!(!f.is_null());
        unsafe {
            assert!((*f).status_code == ffi::evmc_status_code::EVMC_FAILURE);
            assert!((*f).gas_left == 420);
            assert!((*f).output_data.is_null());
            assert!((*f).output_size == 0);
            assert!((*f).create_address.bytes == [0u8; 20]);
            if (*f).release.is_some() {
                (*f).release.unwrap()(f);
            }
        }
    }

    #[test]
    fn into_stack_ffi() {
        let r = ExecutionResult::new(
            ffi::evmc_status_code::EVMC_FAILURE,
            420,
            Some(vec![0xc0, 0xff, 0xee, 0x71, 0x75]),
            Some(ffi::evmc_address { bytes: [0u8; 20] }),
        );

        let f: ffi::evmc_result = r.into();
        unsafe {
            assert!(f.status_code == ffi::evmc_status_code::EVMC_FAILURE);
            assert!(f.gas_left == 420);
            assert!(!f.output_data.is_null());
            assert!(f.output_size == 5);
            assert!(
                std::slice::from_raw_parts(f.output_data, 5) as &[u8]
                    == &[0xc0, 0xff, 0xee, 0x71, 0x75]
            );
            assert!(f.create_address.bytes == [0u8; 20]);
            if f.release.is_some() {
                f.release.unwrap()(&f);
            }
        }
    }

    #[test]
    fn into_stack_ffi_empty_data() {
        let r = ExecutionResult::new(
            ffi::evmc_status_code::EVMC_FAILURE,
            420,
            None,
            Some(ffi::evmc_address { bytes: [0u8; 20] }),
        );

        let f: ffi::evmc_result = r.into();
        unsafe {
            assert!(f.status_code == ffi::evmc_status_code::EVMC_FAILURE);
            assert!(f.gas_left == 420);
            assert!(f.output_data.is_null());
            assert!(f.output_size == 0);
            assert!(f.create_address.bytes == [0u8; 20]);
            if f.release.is_some() {
                f.release.unwrap()(&f);
            }
        }
    }

    #[derive(Clone)]
    pub struct foovm;

    impl EvmcVM for foovm {
        fn init() -> Self {
            foovm
        }

        fn execute(&self, code: &[u8]) -> ExecutionResult {
            unimplemented!()
        }
    }
    evmc_create_vm!(foovm, "0.5");
}
