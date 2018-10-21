use std::ops::{Deref, DerefMut};
use std::slice;

use ejdb_sys;

pub struct TCXString(*mut ejdb_sys::TCXSTR);

impl Drop for TCXString {
    fn drop(&mut self) {
        unsafe {
            ejdb_sys::tcxstrdel(self.0);
        }
    }
}

impl TCXString {
    #[inline]
    pub fn new() -> TCXString {
        TCXString(unsafe { ejdb_sys::tcxstrnew() })
    }

    #[inline]
    pub fn as_raw(&self) -> *mut ejdb_sys::TCXSTR {
        self.0
    }
}

impl Deref for TCXString {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((*self.0).ptr as *const _, (*self.0).size as usize) }
    }
}

impl DerefMut for TCXString {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut((*self.0).ptr as *mut _, (*self.0).size as usize) }
    }
}
