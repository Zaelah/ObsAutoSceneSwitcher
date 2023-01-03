use crate::ffi::OBS;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};
use winapi::{
    ctypes::c_int,
    shared::{
        minwindef::DWORD,
        ntdef::LONG,
        windef::{HWINEVENTHOOK, HWND},
    },
    um::winuser::{GetWindowTextA, SetWinEventHook, UnhookWinEvent},
};

static mut HOOK: Option<HWINEVENTHOOK> = None;
static mut TX: Option<mpsc::Sender<Action>> = None;
static mut EXIT: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

const TITLE_STREAMRAIDERS: &str = "Stream Raiders";
const SCENE_CLIP: &str = "Art-Clip";
const SCENE_BLENDER: &str = "Art-Blender";
const SCENE_UNITY: &str = "Art-Unity";
const SCENE_STREAMRAIDERS: &str = "StreamRaiders";

pub fn start() {
    let (tx, rx) = mpsc::channel::<Action>();
    unsafe {
        TX = Some(tx.clone());
    }

    thread::spawn(move || {
        let mut title_to_scene_name = HashMap::<String, String>::new();
        title_to_scene_name.insert("CLIP STUDIO PAINT".to_string(), SCENE_CLIP.to_string());
        title_to_scene_name.insert("Blender".to_string(), SCENE_BLENDER.to_string());
        title_to_scene_name.insert("Unity".to_string(), SCENE_UNITY.to_string());
        title_to_scene_name.insert(
            TITLE_STREAMRAIDERS.to_string(),
            SCENE_STREAMRAIDERS.to_string(),
        );

        let mut scene_nums = SceneNums::new();
        let clip = scene_nums.get_num(SCENE_CLIP);
        let blender = scene_nums.get_num(SCENE_BLENDER);
        let unity = scene_nums.get_num(SCENE_UNITY);
        let raiders = scene_nums.get_num(SCENE_STREAMRAIDERS);

        scene_nums.add_transition(clip, blender);
        scene_nums.add_transition(clip, unity);
        scene_nums.add_transition(clip, raiders);
        scene_nums.add_transition(blender, clip);
        scene_nums.add_transition(blender, unity);
        scene_nums.add_transition(blender, raiders);
        scene_nums.add_transition(unity, clip);
        scene_nums.add_transition(unity, blender);
        scene_nums.add_transition(unity, raiders);
        scene_nums.add_transition(raiders, clip);
        scene_nums.add_transition(raiders, blender);
        scene_nums.add_transition(raiders, unity);

        let mut prev_title = PrevTitle {
            cur_title: String::new(),
            prev_title: String::new(),
        };

        while let Ok(v) = rx.recv() {
            match v {
                Action::Title(title) => {
                    handle_title(
                        &title,
                        &mut title_to_scene_name,
                        &mut scene_nums,
                        &mut prev_title,
                    );
                }
                Action::Exit => {
                    unsafe {
                        EXIT.store(true, Ordering::SeqCst);
                    }
                    return;
                }
            };
        }
    });

    unsafe {
        let h = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_MINIMIZEEND,
            ptr::null_mut(),
            Some(hook),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        if !h.is_null() {
            HOOK = Some(h);
        }
    };
}

pub fn stop() {
    unsafe {
        if let Some(h) = HOOK {
            let _ = UnhookWinEvent(h);
        }
        if let Some(tx) = &TX {
            let _ = tx.send(Action::Exit);
            loop {
                if EXIT.load(Ordering::SeqCst) {
                    return;
                }
            }
        }
    }
}

fn handle_title(
    dst_title: &String,
    title_to_scene_name: &mut HashMap<String, String>,
    scene_nums: &mut SceneNums,
    prev_title: &mut PrevTitle,
) {
    // record the title of the window we just switched to
    // so we always know where we last were
    prev_title.prev_title = prev_title.cur_title.clone();
    prev_title.cur_title = dst_title.clone();

    for (title, dst_scene_name) in title_to_scene_name.iter() {
        if !dst_title.contains(title) {
            continue;
        }
        let (scenes_by_name, names_by_scene) = OBS::get_scenes_by_name();

        if !scenes_by_name.contains_key(dst_scene_name) {
            continue;
        }

        let dst_scene_id = scenes_by_name[dst_scene_name];
        let src_scene_id = OBS::get_current_scene();
        OBS::release_scene(src_scene_id);

        if !names_by_scene.contains_key(&src_scene_id) {
            continue;
        }

        let src_scene_name = &names_by_scene[&src_scene_id];

        // check if transition (src_scene_name -> dst_scene_name) is valid
        let src_num = scene_nums.get_num(src_scene_name);
        let dst_num = scene_nums.get_num(dst_scene_name);

        if scene_nums.has_transition(src_num, dst_num) {
            OBS::set_scene(dst_scene_id);
            return;
        }
    }

    // if we're here, we didn't do a transition
    // special handling: transitioning to stream raiders is always valid
    if dst_title.contains(TITLE_STREAMRAIDERS) {
        let (scenes_by_name, names_by_scene) = OBS::get_scenes_by_name();
        let src_scene_id = OBS::get_current_scene();
        OBS::release_scene(src_scene_id);

        if scenes_by_name.contains_key(SCENE_STREAMRAIDERS) {
            let dst_scene_id = scenes_by_name[SCENE_STREAMRAIDERS];
            OBS::set_scene(dst_scene_id);
        }

        // whatever title + scene combination we just came from is now valid for
        // a return transition from stream raiders
        if names_by_scene.contains_key(&src_scene_id) {
            let src_scene_name = &names_by_scene[&src_scene_id];
            let src_title = &prev_title.prev_title;

            // whatever's title -> scene name
            title_to_scene_name.insert(src_title.clone(), src_scene_name.clone());

            let whatever_num = scene_nums.get_num(src_scene_name);
            let raiders_num = scene_nums.get_num(SCENE_STREAMRAIDERS);

            // stream raiders -> whatever
            scene_nums.add_transition(raiders_num, whatever_num);
        }
    }
}

enum Action {
    Title(String),
    Exit,
}

struct PrevTitle {
    cur_title: String,
    prev_title: String,
}

struct Transition {
    src: u16,
    dst: u16,
}

struct SceneNums {
    transitions: Vec<Transition>,
    name_to_u16: HashMap<String, u16>,
    next_num: u16,
}

impl SceneNums {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            name_to_u16: HashMap::new(),
            next_num: 1,
        }
    }

    pub fn get_num(&mut self, scene_name: &str) -> u16 {
        if self.name_to_u16.contains_key(scene_name) {
            self.name_to_u16[scene_name]
        } else {
            let num = self.next_num;
            self.next_num += 1;
            self.name_to_u16.insert(scene_name.to_string(), num);
            num
        }
    }

    pub fn add_transition(&mut self, src: u16, dst: u16) {
        if src != dst && !self.has_transition(src, dst) {
            self.transitions.push(Transition { src, dst });
        }
    }

    pub fn has_transition(&self, src: u16, dst: u16) -> bool {
        for t in &self.transitions {
            if t.src == src && t.dst == dst {
                return true;
            }
        }
        false
    }
}

const EVENT_SYSTEM_FOREGROUND: DWORD = 0x0003;
const EVENT_SYSTEM_MINIMIZEEND: DWORD = 0x0017;
const WINEVENT_OUTOFCONTEXT: DWORD = 0x0000;
const WINEVENT_SKIPOWNPROCESS: DWORD = 0x0002;

unsafe extern "system" fn hook(
    _: HWINEVENTHOOK,
    event: DWORD,
    hwnd: HWND,
    _: LONG,
    _: LONG,
    _: DWORD,
    _: DWORD,
) {
    if event != EVENT_SYSTEM_FOREGROUND && event != EVENT_SYSTEM_MINIMIZEEND {
        return;
    }
    if hwnd.is_null() {
        return;
    }

    let mut buf: [u8; 1024] = [0; 1024];
    let n: c_int = GetWindowTextA(hwnd, buf.as_mut_ptr() as *mut i8, buf.len() as i32);
    if n > 0 {
        let title = String::from_utf8_lossy(&buf[0..(n as usize)]).to_string();
        if let Some(tx) = &TX {
            let _ = tx.send(Action::Title(title));
        }
    }
}
