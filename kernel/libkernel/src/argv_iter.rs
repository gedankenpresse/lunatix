struct CmdArgIter {
    argc: u32,
    current: u32,
    argv: *const *const core::ffi::c_char,
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

pub fn arg_iter(
    argc: u32,
    argv: *const *const core::ffi::c_char,
) -> impl Iterator<Item = &'static str> {
    CmdArgIter {
        argc,
        argv,
        current: 0,
    }
}
