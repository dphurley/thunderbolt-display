use std::ffi::CString;

#[repr(C)]
struct VirtualDisplayOpaque {
    _private: [u8; 0],
}

#[repr(C)]
struct VirtualDisplayResult {
    handle: *mut VirtualDisplayOpaque,
    display_id: u32,
}

#[link(name = "virtual_display", kind = "static")]
extern "C" {
    fn vt_virtual_display_create(
        width: u32,
        height: u32,
        ppi: u32,
        hi_dpi: bool,
        name_utf8: *const i8,
    ) -> VirtualDisplayResult;
    fn vt_virtual_display_destroy(display: *mut VirtualDisplayOpaque);
}

#[derive(Debug)]
pub struct VirtualDisplay {
    handle: *mut VirtualDisplayOpaque,
    display_id: u32,
}

impl VirtualDisplay {
    pub fn create(
        width: u32,
        height: u32,
        ppi: u32,
        hi_dpi: bool,
        name: &str,
    ) -> Result<Self, String> {
        let name_c = CString::new(name).map_err(|_| "invalid display name")?;
        let result = unsafe {
            vt_virtual_display_create(width, height, ppi, hi_dpi, name_c.as_ptr())
        };
        if result.handle.is_null() || result.display_id == 0 {
            return Err("failed to create virtual display".to_string());
        }
        Ok(Self {
            handle: result.handle,
            display_id: result.display_id,
        })
    }

    pub fn display_id(&self) -> u32 {
        self.display_id
    }
}

impl Drop for VirtualDisplay {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { vt_virtual_display_destroy(self.handle) };
        }
    }
}
