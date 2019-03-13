use evmc_sys as ffi;

/// EVMC message (call) kind.
pub type MessageKind = ffi::evmc_call_kind;

/// EVMC message (call) flags.
pub type MessageFlags = ffi::evmc_flags;

/// EVMC status code.
pub type StatusCode = ffi::evmc_status_code;

/// EVMC storage status.
pub type StorageStatus = ffi::evmc_storage_status;

/// EVMC VM revision.
pub type Revision = ffi::evmc_revision;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_kind() {
        //        assert_eq!(MessageKind::EVMC_CALL, ffi::evmc_call_kind::EVMC_CALL);
        //        assert_eq!(MessageKind::EVMC_CREATE, ffi::evmc_call_kind::EVMC_CREATE);
        let x = MessageKind::EVMC_ISTANBUL;
        //      assert_eq!(Revision::EVMC_ISTANBUL, ffi::evmc_revision::EVMC_ISTANBUL);
    }
}
