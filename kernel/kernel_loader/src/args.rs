//! Kernel Argument handling
//!
//! Typically U-Boot passes through kernel parameters via an `argc`, `argv` pair.
//! The code in this module parses relevant kernel-loader arguments from that iterator.

use core::ffi::CStr;

/// An iterator over an *argc*, *argv* pair.
///
/// This is a typical c-style pattern for passing a list of argument strings.
/// It works like this:
///
/// - `argc` describes how many arguments are passed (think `argc = argument_count`)
/// - `argv` points to the start of an array of pointers to those arguments.
///   Each argument is expected to be a null-terminated string (CStr) and the array items point to the start of each arguments string.
///
/// To iterate over all arguments, one needs to dereference and add `1` to `argv` exactly `argc` times.
pub struct CmdArgIter {
    argc: u32,
    current: u32,
    argv: *const *const core::ffi::c_char,
}

impl CmdArgIter {
    /// Create a new iterator from the given `argc`, `argv` pair
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
        let current = self.current;
        self.current += 1;
        let cstr = unsafe { *self.argv.add(current as usize) };
        let cs = unsafe { CStr::from_ptr(cstr) };
        let s = cs
            .to_str()
            .expect("A kernel parameter is not a valid string");
        return Some(s);
    }
}

/// Arguments given to kernel_loader packed into a struct
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct LoaderArgs {
    /// The address of the *flattened device tree* (in physical memory) which u-boot prepared for us and which
    /// describes the hardware lunatix was booted on.
    pub phys_fdt_addr: *const u8,

    /// The address of the kernel binary image (in physical memory).
    /// This image is usually placed there by qemu or u-boot before jumping into the kernel_loader.
    pub image_addr: *const u8,

    /// The size of the kernel image in bytes.
    pub image_size: usize,
}

impl LoaderArgs {
    /// Parse a semantic `Args` struct from an iterator over raw arguments
    pub fn from_args(args: impl Iterator<Item = &'static str>) -> Self {
        log::trace!("parsing kernel parameters");

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

        // set sane argument defaults
        if image_size.is_none() {
            log::warn!("no image_size= (size of the actual kernel image in bytes) kernel argument given; assuming 2MB");
            const MB: usize = 1024 * 1024;
            image_size = Some(2 * MB);
        }

        Self {
            phys_fdt_addr: phys_fdt_addr
                .expect("no fdt_addr= (address of the device tree blob) kernel argument given"),
            image_addr: image_addr
                .expect("no image_addr= (image of the actual kernel) kernel argument given"),
            image_size: image_size.unwrap(),
        }
    }

    /// Get a slice to the in-memory kernel binary as indicated by the argument
    pub fn get_kernel_bin(&self) -> &[u8] {
        // Safety: This is as safe as it gets because we receive those arguments from our bootloader which we trust
        unsafe { core::slice::from_raw_parts(self.image_addr, self.image_size) }
    }
}
