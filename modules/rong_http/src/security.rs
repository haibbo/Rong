use rong::*;
use std::cell::RefCell;

thread_local! {
    static NETWORK_ACCESS_GUARD: RefCell<Box<dyn NetworkAccessGuard>> = RefCell::new(Box::new(DefaultNetworkAccessGuard));
}

pub trait NetworkAccessGuard {
    /// Check if access to the given domain is allowed
    /// Returns Ok(()) if access is granted, Err with error message if denied
    fn check_access(&self, domain: &str) -> JSResult<()>;
}

#[derive(Default)]
struct DefaultNetworkAccessGuard;

impl NetworkAccessGuard for DefaultNetworkAccessGuard {
    fn check_access(&self, _domain: &str) -> JSResult<()> {
        Ok(()) // Allow all domains by default
    }
}

/// Set a custom network access guard for the current thread.
///
/// The fetch module forwards this policy into `rong_rt::http::RequestOptions`,
/// so the same guard is enforced by the underlying runtime request path.
/// This allows applications to implement custom network access control policies
///
/// # Example
/// ```rust
/// use rong_http::{set_network_access_guard, NetworkAccessGuard};
/// use rong::JSResult;
///
/// struct RestrictedNetworkGuard;
///
/// impl NetworkAccessGuard for RestrictedNetworkGuard {
///     fn check_access(&self, domain: &str) -> JSResult<()> {
///         if domain == "api.example.com" || domain.ends_with(".example.com") {
///             Ok(())
///         } else {
///             Err(rong::HostError::new(rong::error::E_PERMISSION_DENIED, "Domain access denied").into())
///         }
///     }
/// }
///
/// set_network_access_guard(Box::new(RestrictedNetworkGuard));
/// ```
pub fn set_network_access_guard(guard: Box<dyn NetworkAccessGuard>) {
    NETWORK_ACCESS_GUARD.with(|g| {
        *g.borrow_mut() = guard;
    });
}

/// Scoped network access guard setter.
/// Restores the previous guard when the returned scope is dropped.
pub fn set_network_access_guard_scoped(
    guard: Box<dyn NetworkAccessGuard>,
) -> NetworkAccessGuardScope {
    let prev = NETWORK_ACCESS_GUARD.with(|g| std::mem::replace(&mut *g.borrow_mut(), guard));
    NetworkAccessGuardScope { prev: Some(prev) }
}

pub struct NetworkAccessGuardScope {
    prev: Option<Box<dyn NetworkAccessGuard>>,
}

impl Drop for NetworkAccessGuardScope {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            NETWORK_ACCESS_GUARD.with(|g| {
                *g.borrow_mut() = prev;
            });
        }
    }
}

/// Grant network access for a specific domain
/// This function checks if the current network access guard allows access to the given domain
pub fn grant_network_access(domain: &str) -> JSResult<()> {
    NETWORK_ACCESS_GUARD.with(|guard| guard.borrow().check_access(domain))
}
