//! Workspace contract: every production crate reports the same API version and
//! `todo-core` exposes a complete `BuildInfo`.

use todo_core::{API_VERSION as CORE_API, BuildInfo};
use todo_crypto::API_VERSION as CRYPTO_API;
use todo_discovery::API_VERSION as DISCOVERY_API;
use todo_domain::API_VERSION as DOMAIN_API;
use todo_protocol::API_VERSION as PROTOCOL_API;
use todo_storage::API_VERSION as STORAGE_API;
use todo_uniffi::API_VERSION as UNIFFI_API;

#[test]
fn all_production_crates_share_api_version() {
    assert_eq!(DOMAIN_API, CORE_API);
    assert_eq!(STORAGE_API, CORE_API);
    assert_eq!(CRYPTO_API, CORE_API);
    assert_eq!(PROTOCOL_API, CORE_API);
    assert_eq!(DISCOVERY_API, CORE_API);
    assert_eq!(UNIFFI_API, CORE_API);
}

#[test]
fn core_build_info_is_complete() {
    let info = BuildInfo::current();
    assert_eq!(info.core_api, CORE_API);
    assert_eq!(info.schema, 1);
    assert_eq!(info.protocol_major, 1);
    assert_eq!(info.protocol_minor, 0);
}
