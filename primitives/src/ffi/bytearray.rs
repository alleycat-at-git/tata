use std::{convert::TryInto, mem::ManuallyDrop, string::FromUtf8Error};

/// FFI representation of array of bytes
///
/// The memory in the array should be dropped manually (using `free` method).
/// The Into<_> traits frees the memory automatically
#[repr(C)]
pub struct ByteArray {
    data: *mut u8,
    len: usize,
}

impl From<Vec<u8>> for ByteArray {
    fn from(v: Vec<u8>) -> Self {
        let mut v = ManuallyDrop::new(v);
        ByteArray {
            data: v.as_mut_ptr(),
            len: v.len(),
        }
    }
}

impl Into<Vec<u8>> for ByteArray {
    fn into(self) -> Vec<u8> {
        unsafe {
            let res = std::slice::from_raw_parts(self.data, self.len).to_vec();
            self.free();
            res
        }
    }
}

impl From<String> for ByteArray {
    fn from(v: String) -> Self {
        let bytes = v.as_bytes().to_vec();
        bytes.into()
    }
}

impl TryInto<String> for ByteArray {
    type Error = FromUtf8Error;

    fn try_into(self) -> Result<String, FromUtf8Error> {
        let bytes: Vec<u8> = self.into();
        String::from_utf8(bytes)
    }
}

impl ByteArray {
    pub unsafe fn free(self) {
        let s = std::slice::from_raw_parts_mut(self.data, self.len);
        let s = s.as_mut_ptr();
        Box::from_raw(s);
    }
}
