//! Lock-free SPSC ring buffer for audio samples

use std::sync::atomic::{AtomicUsize, Ordering};

/// Cache-line aligned SPSC (Single Producer Single Consumer) ring buffer
///
/// Designed for zero-allocation audio streaming between threads.
#[repr(C)]
pub struct RingBuffer<T> {
    buffer: Box<[T]>,
    capacity: usize,
    // Write position on its own cache line to avoid false sharing
    write_pos: CacheAligned<AtomicUsize>,
    // Read position on its own cache line
    read_pos: CacheAligned<AtomicUsize>,
}

/// Cache-line aligned wrapper (64 bytes on most architectures)
#[repr(C, align(64))]
struct CacheAligned<T> {
    value: T,
}

impl<T> CacheAligned<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Default + Clone> RingBuffer<T> {
    /// Create a new ring buffer with the specified capacity
    ///
    /// Capacity will be rounded up to the next power of two for efficient modulo operations.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        let buffer = vec![T::default(); capacity].into_boxed_slice();

        Self {
            buffer,
            capacity,
            write_pos: CacheAligned::new(AtomicUsize::new(0)),
            read_pos: CacheAligned::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the capacity of the buffer
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of items available for reading
    #[inline]
    pub fn len(&self) -> usize {
        let write = self.write_pos.value.load(Ordering::Acquire);
        let read = self.read_pos.value.load(Ordering::Acquire);
        write.wrapping_sub(read)
    }

    /// Returns true if the buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of free slots for writing
    #[inline]
    pub fn free(&self) -> usize {
        self.capacity - self.len()
    }

    /// Push a single item to the buffer
    ///
    /// Returns true if successful, false if buffer is full.
    #[inline]
    pub fn push(&self, item: T) -> bool
    where
        T: Copy,
    {
        let write = self.write_pos.value.load(Ordering::Relaxed);
        let read = self.read_pos.value.load(Ordering::Acquire);

        if write.wrapping_sub(read) >= self.capacity {
            return false; // Buffer full
        }

        let index = write & (self.capacity - 1);
        // SAFETY: We have exclusive write access and index is within bounds
        unsafe {
            let ptr = self.buffer.as_ptr() as *mut T;
            ptr.add(index).write(item);
        }

        self.write_pos.value.store(write.wrapping_add(1), Ordering::Release);
        true
    }

    /// Push multiple items to the buffer
    ///
    /// Returns the number of items actually written.
    pub fn push_slice(&self, items: &[T]) -> usize
    where
        T: Copy,
    {
        let mut written = 0;
        for &item in items {
            if !self.push(item) {
                break;
            }
            written += 1;
        }
        written
    }

    /// Pop a single item from the buffer
    #[inline]
    pub fn pop(&self) -> Option<T>
    where
        T: Copy,
    {
        let read = self.read_pos.value.load(Ordering::Relaxed);
        let write = self.write_pos.value.load(Ordering::Acquire);

        if read == write {
            return None; // Buffer empty
        }

        let index = read & (self.capacity - 1);
        // SAFETY: We have exclusive read access and index is within bounds
        let item = unsafe {
            let ptr = self.buffer.as_ptr();
            ptr.add(index).read()
        };

        self.read_pos.value.store(read.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    /// Read multiple items into a slice
    ///
    /// Returns the number of items actually read.
    pub fn pop_slice(&self, output: &mut [T]) -> usize
    where
        T: Copy,
    {
        let mut read_count = 0;
        for slot in output.iter_mut() {
            match self.pop() {
                Some(item) => {
                    *slot = item;
                    read_count += 1;
                }
                None => break,
            }
        }
        read_count
    }

    /// Clear all items from the buffer
    pub fn clear(&self) {
        let write = self.write_pos.value.load(Ordering::Acquire);
        self.read_pos.value.store(write, Ordering::Release);
    }
}

// SAFETY: RingBuffer can be safely shared between threads
// The atomic operations ensure proper synchronization
unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Send> Sync for RingBuffer<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let buffer: RingBuffer<f32> = RingBuffer::new(8);

        assert!(buffer.push(1.0));
        assert!(buffer.push(2.0));
        assert_eq!(buffer.len(), 2);

        assert_eq!(buffer.pop(), Some(1.0));
        assert_eq!(buffer.pop(), Some(2.0));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_capacity_rounding() {
        let buffer: RingBuffer<u8> = RingBuffer::new(10);
        assert_eq!(buffer.capacity(), 16); // Rounded to next power of 2
    }

    #[test]
    fn test_full_buffer() {
        let buffer: RingBuffer<i32> = RingBuffer::new(4);

        assert!(buffer.push(1));
        assert!(buffer.push(2));
        assert!(buffer.push(3));
        assert!(buffer.push(4));
        assert!(!buffer.push(5)); // Should fail, buffer full

        assert_eq!(buffer.pop(), Some(1));
        assert!(buffer.push(5)); // Now there's space
    }
}
