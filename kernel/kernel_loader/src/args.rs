/// An iterator over an *argc*, *argv* pair
pub struct CmdArgIter {
    argc: u32,
    current: u32,
    argv: *const *const core::ffi::c_char,
}

impl CmdArgIter {
    pub fn from_argc_argv(argc: u32, argv: *const *const core::ffi::c_char) -> Self {
        CmdArgIter {
            argc,
            argv,
            current: 0,
        }
    }
}

impl Iterator for CmdArgIter {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.argc {
            return None;
        }
        let i = self.current;
        self.current += 1;
        let cstr = unsafe { *self.argv.offset(i as isize) };
        use core::ffi::CStr;
        let cs = unsafe { CStr::from_ptr(cstr) };
        let s = cs.to_str().unwrap();
        return Some(s);
    }
}

/// Arguments given to kernel_loader packed into a struct
pub struct LoaderArgs {
    pub phys_fdt_addr: *const u8,
    pub image_addr: *const u8,
    pub image_size: Option<usize>,
}

impl LoaderArgs {
    /// Parse a semantic `Args` struct from an iterator over raw arguments
    pub fn from_args(args: impl Iterator<Item = &'static str>) -> Self {
        let mut phys_fdt_addr = None;
        let mut image_addr = None;
        let mut image_size = None;
        for arg in args {
            if let Some(addr_s) = arg.strip_prefix("fdt_addr=") {
                let addr =
                    usize::from_str_radix(addr_s, 16).expect("fdt_addr should be in base 16");
                phys_fdt_addr = Some(addr as *const u8);
            }
            if let Some(addr_s) = arg.strip_prefix("image_addr=") {
                let addr =
                    usize::from_str_radix(addr_s, 16).expect("image_addr should be in base 16");
                image_addr = Some(addr as *const u8);
            }
            if let Some(size_s) = arg.strip_prefix("image_size=") {
                let size =
                    usize::from_str_radix(size_s, 16).expect("image size should be in base 16");
                image_size = Some(size);
            }
        }

        Self {
            phys_fdt_addr: phys_fdt_addr.expect("no fdt_addr given"),
            image_addr: image_addr.expect("no kernel image addr given"),
            image_size,
        }
    }

    /// Get a slice to the in-memory kernel binary as indicated by the argument
    pub fn get_kernel_bin(&self) -> &[u8] {
        const MB: usize = 1024 * 1024;
        unsafe { core::slice::from_raw_parts(self.image_addr, self.image_size.unwrap_or(2 * MB)) }
    }
}
