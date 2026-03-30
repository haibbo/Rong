#[cfg(target_env = "ohos")]
mod class;
#[cfg(target_env = "ohos")]
mod context;
#[cfg(target_env = "ohos")]
mod runtime;
#[cfg(target_env = "ohos")]
mod value;

#[cfg(target_env = "ohos")]
mod arkjs {
    // Native low-level bindings for Harmony Ark JS
    pub use rong_arkjs_sys::*;
}

#[cfg(target_env = "ohos")]
pub use context::ArkJSContext;
#[cfg(target_env = "ohos")]
pub use runtime::{ArkJSRuntime, HarmonyArkJS};
#[cfg(target_env = "ohos")]
pub use value::ArkJSValue;

#[cfg(not(target_env = "ohos"))]
pub struct HarmonyArkJS;
#[cfg(not(target_env = "ohos"))]
pub struct ArkJSRuntime;
#[cfg(not(target_env = "ohos"))]
pub struct ArkJSContext;
#[cfg(not(target_env = "ohos"))]
pub struct ArkJSValue;
