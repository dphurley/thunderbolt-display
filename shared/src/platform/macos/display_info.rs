#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DisplayInfo {
    pub display_id: u32,
    pub width: u32,
    pub height: u32,
    pub is_main: u32,
}

#[link(name = "display_info", kind = "static")]
extern "C" {
    fn vt_display_list(out_displays: *mut DisplayInfo, max_displays: u32) -> u32;
}

pub fn list_displays() -> Vec<DisplayInfo> {
    let mut displays: [DisplayInfo; 16] = [DisplayInfo {
        display_id: 0,
        width: 0,
        height: 0,
        is_main: 0,
    }; 16];

    let count = unsafe { vt_display_list(displays.as_mut_ptr(), displays.len() as u32) };
    displays[..(count as usize)].to_vec()
}
