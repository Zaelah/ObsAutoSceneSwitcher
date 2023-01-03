use crate::ffi::obs_module_t;
use std::ptr;

mod ffi;
mod implementation;

static mut OBS_MODULE: Option<obs_module_t> = None;

#[no_mangle]
pub unsafe extern "C" fn obs_module_set_pointer(ptr: obs_module_t) {
    OBS_MODULE = Some(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn obs_current_module() -> obs_module_t {
    match OBS_MODULE {
        Some(m) => m,
        None => ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_load() -> bool {
    implementation::start();
    true
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_unload() {
    implementation::stop();
    OBS_MODULE = None;
}

const LIBOBS_API_MAJOR_VER: u32 = 28;
const LIBOBS_API_MINOR_VER: u32 = 1;
const LIBOBS_API_PATCH_VER: u32 = 2;
const LIBOBS_API_VER: u32 =
    (LIBOBS_API_MAJOR_VER << 24) | (LIBOBS_API_MINOR_VER << 16) | LIBOBS_API_PATCH_VER;

#[no_mangle]
pub unsafe extern "C" fn obs_module_ver() -> u32 {
    LIBOBS_API_VER
}
