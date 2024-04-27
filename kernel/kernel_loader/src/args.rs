//! Kernel Argument handling
//!
//! Typically, U-Boot passes through kernel parameters via an `argc`, `argv` pair.
//! The code in this module parses relevant kernel-loader arguments from that iterator.

use allocators::{AllocInit, Allocator};
use core::alloc::Layout;
use core::ffi::CStr;
use core::mem;
use core::mem::MaybeUninit;
use riscv::mem::PageTable;

/// A storage struct for the data pointed to by `argc, argv`
#[derive(Debug, Clone)]
struct ArgumentStore {
    data: [u8; 64],
    idx: [usize; 4],
}

/// The location into which argument data is temporarily stored
static mut TMP_STORE: ArgumentStore = ArgumentStore {
    data: [0; 64],
    idx: [0; 4],
};

/// Copy the data pointed to by `argc, argv` in a temporary, internal location and return a new `argv` pointer.
///
/// # Safety
/// This function must not be called more than once.
///
/// This function must never be called in a concurrent environment.
///
/// The data pointed to by `argc, argv` must not lie in the internal storage location.
pub unsafe fn inline_args(
    argc: u32,
    argv: *const *const core::ffi::c_char,
) -> *const *const core::ffi::c_char {
    log::debug!("moving argument data to internal, temporary location");
    assert!(
        argc as usize <= TMP_STORE.idx.len(),
        "kernel_loader can handle at most {} arguments passed via argc, argv ({} were given)",
        TMP_STORE.idx.len(),
        argc
    );

    // iterate over all arguments and move them into the internal store
    {
        let mut store = &mut TMP_STORE;
        let mut data_free_start = 0;
        for i in 0..argc {
            let arg_ptr = *argv.add(i as usize).as_ref().unwrap();
            let arg_str = CStr::from_ptr(arg_ptr).to_bytes_with_nul();
            assert!(data_free_start + arg_str.len() <= TMP_STORE.data.len(), "kernel_loader can handle at most {} chars as argc, argv arguments (at least {} were given)", TMP_STORE.data.len(), data_free_start + arg_str.len());

            store.data[data_free_start..data_free_start + arg_str.len()].copy_from_slice(arg_str);
            store.idx[i as usize] = (&store.data[data_free_start]) as *const u8 as usize;
            data_free_start += arg_str.len()
        }
    }

    // return a new pointer that points to the internal store
    TMP_STORE.idx.as_ptr() as *const *const core::ffi::c_char
}

/// Copy data from the temporary, internal location to
///
/// # Safety
/// This function mus be called after `inline_args()` has been successfully called.
pub unsafe fn copy_to_allocated_mem<'a>(
    alloc: &impl Allocator<'a>,
) -> *const *const core::ffi::c_char {
    log::debug!("moving argument data to allocated memory");

    // allocate memory and data from TMP_STORE into it
    let ptr = alloc
        .allocate(Layout::new::<ArgumentStore>(), AllocInit::Uninitialized)
        .expect("Could not allocate memory for root page table")
        .as_mut_ptr()
        .cast::<MaybeUninit<ArgumentStore>>();
    core::ptr::copy_nonoverlapping(
        (&TMP_STORE) as *const ArgumentStore as *const MaybeUninit<ArgumentStore>,
        ptr,
        1,
    );

    // return a new argv pointer
    log::trace!("argc, argv data is now stored at {ptr:p}");
    let store = ptr.as_ref().unwrap().assume_init_ref();
    store.idx.as_ptr() as *const *const core::ffi::c_char
}

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
