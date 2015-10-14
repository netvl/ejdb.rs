use std::marker::PhantomData;

use libc::c_int;

use ejdb_sys;

pub struct TCList<T>(*mut ejdb_sys::TCLIST, PhantomData<T>);

impl<T> Drop for TCList<T> {
    fn drop(&mut self) {
        unsafe { ejdb_sys::tclistdel(self.0) }
    }
}

impl<T> TCList<T> {
    pub unsafe fn from_ptr(tclist: *mut ejdb_sys::TCLIST) -> TCList<T> {
        TCList(tclist, PhantomData)
    }

    #[inline]
    pub fn len(&self) -> c_int {
        unsafe { ejdb_sys::tclistnum(self.0) }
    }

    #[inline]
    pub fn index_unchecked(&self, idx: c_int) -> *mut T {
        let mut item_size = 0;
        unsafe {
            ejdb_sys::tclistval(self.0 as *const _, idx, &mut item_size) as *const T as *mut T
        }
    }

    pub fn iter(&self) -> TCListIter<T> { TCListIter(self, 0) }
}

pub struct TCListIter<'a, T: 'a>(&'a TCList<T>, c_int);

impl<'a, T> Iterator for TCListIter<'a, T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= self.0.len() {
            None
        } else {
            let result = self.0.index_unchecked(self.1);
            self.1 += 1;
            Some(result)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len() as usize, Some(self.0.len() as usize))
    }
}
