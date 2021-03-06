//! Contains ring vector

/// Ring vector uses limited size buffer for storing elements.
/// Effectively it's a window containing last `size` pushed elements.
pub struct RingVec<T> {
    size: usize,
    pos: usize,
    data: Vec<T>,
}

impl<T> RingVec<T> {
    pub fn new(size: usize) -> Self {
        RingVec {
            size,
            pos: 0,
            data: Vec::with_capacity(size),
        }
    }

    /// Push an element. If the `size` limit is reached
    /// it will start overwriting elements in the beginning
    pub fn push(&mut self, elem: T) {
        self.pos += 1;
        if self.data.len() != self.size {
            self.data.push(elem);
        } else {
            self.data[self.pos % self.size] = elem;
        }
    }

    /// Get the element. If the `pos` is out of bounds (not in the last `size` pushed elements)
    /// returns `None`.
    pub fn get(&self, pos: usize) -> Option<&T> {
        if (pos >= self.pos) || (pos + self.size < self.pos) {
            return None;
        }
        Some(
            self.data
                .get(pos % self.size)
                .expect("The element is in the buffer; qed"),
        )
    }
}
