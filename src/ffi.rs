use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::ptr;

pub struct OBS;

impl OBS {
    pub fn get_scenes_by_name() -> (HashMap<String, obs_source_t>, HashMap<obs_source_t, String>) {
        let mut list = obs_frontend_source_list {
            array: ptr::null(),
            num: 0,
            capacity: 0,
        };

        unsafe {
            obs_frontend_get_scenes(&mut list);
        }

        let mut by_name = HashMap::new();
        let mut by_scene = HashMap::new();
        for i in 0..list.num {
            let scene = unsafe { *list.array.offset(i as isize) };
            let cstr = unsafe { CStr::from_ptr(obs_source_get_name(scene)) };
            let name = String::from_utf8_lossy(cstr.to_bytes()).to_string();
            by_name.insert(name.clone(), scene);
            by_scene.insert(scene, name);
            Self::release_scene(scene);
        }

        if list.array != ptr::null() {
            unsafe {
                bfree(list.array);
            }
        }

        (by_name, by_scene)
    }

    pub fn get_current_scene() -> obs_source_t {
        unsafe { obs_frontend_get_current_scene() }
    }

    pub fn set_scene(scene: obs_source_t) {
        unsafe {
            obs_frontend_set_current_scene(scene);
        }
    }

    pub fn release_scene(scene: obs_source_t) {
        unsafe {
            obs_source_release(scene);
        }
    }
}

#[allow(non_camel_case_types)]
pub type obs_module_t = *const c_void;
#[allow(non_camel_case_types)]
pub type obs_source_t = usize; // assuming we're on a platform where usize = ptr size

#[repr(C)]
struct obs_frontend_source_list {
    array: *const obs_source_t,
    num: usize,
    capacity: usize,
}

#[link(name = "obs-frontend-api")]
extern "C" {
    fn obs_frontend_get_current_scene() -> obs_source_t;
    fn obs_frontend_set_current_scene(scene: obs_source_t);
    fn obs_frontend_get_scenes(list: *mut obs_frontend_source_list);
}

#[link(name = "obs")]
extern "C" {
    fn bfree(ptr: *const obs_source_t);
    fn obs_source_get_name(source: obs_source_t) -> *const c_char;
    fn obs_source_release(source: obs_source_t);
}
