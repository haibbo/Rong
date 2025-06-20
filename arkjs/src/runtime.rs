use crate::{ArkJSContext, ArkJSValue, arkjs};
use rong_core::{JSEngine, JSRuntimeImpl};
use std::ptr;

pub struct ArkJSRuntime {
    raw: arkjs::JSVM_VM,
}

impl JSRuntimeImpl for ArkJSRuntime {
    type RawRuntime = arkjs::JSVM_VM;
    type Context = ArkJSContext;

    fn new() -> Self {
        let mut vm: arkjs::JSVM_VM = ptr::null_mut();
        let init_options = arkjs::JSVM_InitOptions {
            externalReferences: ptr::null(),
            argc: ptr::null_mut(),
            argv: ptr::null_mut(),
            removeFlags: false,
        };

        unsafe {
            let status = arkjs::OH_JSVM_Init(&init_options);
            if status != arkjs::JSVM_Status_JSVM_OK {
                panic!("Failed to initialize Ark JS VM: {:?}", status);
            }

            let create_options = arkjs::JSVM_CreateVMOptions {
                maxOldGenerationSize: 0,
                maxYoungGenerationSize: 0,
                initialOldGenerationSize: 0,
                initialYoungGenerationSize: 0,
                snapshotBlobData: ptr::null(),
                snapshotBlobSize: 0,
                isForSnapshotting: false,
            };

            let status = arkjs::OH_JSVM_CreateVM(&create_options, &mut vm);
            if status != arkjs::JSVM_Status_JSVM_OK {
                panic!("Failed to create Ark JS VM: {:?}", status);
            }
        }

        Self { raw: vm }
    }

    fn to_raw(&self) -> Self::RawRuntime {
        self.raw
    }

    fn run_gc(&self) {
        // Harmony Ark JS doesn't expose direct GC control like this
        // GC is managed automatically by the runtime
    }
}

impl Drop for ArkJSRuntime {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                arkjs::OH_JSVM_DestroyVM(self.raw);
            }
        }
    }
}

pub struct HarmonyArkJS;

impl JSEngine for HarmonyArkJS {
    type Value = ArkJSValue;
    type Context = ArkJSContext;
    type Runtime = ArkJSRuntime;

    fn name() -> &'static str {
        "HarmonyArkJS"
    }

    fn version() -> String {
        String::from("ArkJS 1.0.0")
    }
}
