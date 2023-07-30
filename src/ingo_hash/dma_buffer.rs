use std::mem;

#[repr(C, align(4096))]
pub struct Align4K([u8; 4096]);

#[derive(Debug)]
pub struct DmaBuffer(Vec<u8>);

impl DmaBuffer {
    pub fn new<T>(n_bytes: usize) -> Self {
        Self(unsafe { aligned_vec::<T>(n_bytes) })
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.0.extend_from_slice(slice);
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }

    pub fn get(&self) -> &Vec<u8> {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    pub fn set_len(&mut self, num: usize) {
        unsafe { self.0.set_len(num) }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub unsafe fn aligned_vec<T>(n_bytes: usize) -> Vec<u8> {
    let n_units = (n_bytes / mem::size_of::<T>()) + 1;

    let mut aligned: Vec<T> = Vec::with_capacity(n_units);

    let ptr = aligned.as_mut_ptr();
    let len_units = aligned.len();
    let cap_units = aligned.capacity();

    mem::forget(aligned);

    Vec::from_raw_parts(
        ptr as *mut u8,
        len_units * mem::size_of::<T>(),
        cap_units * mem::size_of::<T>(),
    )
}
