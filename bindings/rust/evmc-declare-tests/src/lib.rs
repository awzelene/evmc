use evmc_vm::EvmcVm;
use evmc_vm::ExecutionResult;
use evmc_vm::ExecutionContext;
#[macro_use]
use evmc_declare::evmc_declare_vm;

#[evmc_declare_vm("FOO VM", "0.1.0", "ewasm")]
pub struct FooVM {
    a: i32,
}

impl EvmcVm for FooVM {
    fn init() -> Self {
        FooVM { 
            a: 105023,
        }
    }

    fn execute(&self, code: &[u8], context: &ExecutionContext) -> ExecutionResult {
        ExecutionResult::new(evmc_sys::evmc_status_code::EVMC_SUCCESS, 235117, None, None)
    }
}
