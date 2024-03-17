// The Bitmap struct represents a bitmap, a compact and efficient data structure to store binary data (bits).
// Each bit in the bitmap can represent two states, typically used for flags, presence/absence, or other binary indicators.
pub struct Bitmap {
    pub(crate) buffer: *mut u8, // Pointer to the memory buffer that stores the bits of the bitmap.
    pub(crate) size: usize, // The size of the buffer in bytes, indicating how many bits are managed by this bitmap.
}

impl Bitmap {
    /// Creates a new `Bitmap` instance.
    ///
    /// This function constructs a new `Bitmap` by providing a memory buffer and its size.
    /// The buffer serves as the underlying storage for the bitmap's bits.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable pointer to a u8 array representing the bitmap's storage in memory.
    /// * `size` - The size of the buffer in bytes, determining the capacity of the bitmap.
    ///
    /// # Returns
    ///
    /// A new instance of `Bitmap`.
    pub const fn new(buffer: *mut u8, size: usize) -> Bitmap {
        Bitmap { buffer, size }
    }

    /// Gets the value of a bit at a specific index in the bitmap.
    ///
    /// This function is marked unsafe as it performs raw pointer arithmetic and dereferencing without bounds checking.
    /// It retrieves the boolean value (true or false) of the bit at the given index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the bit to retrieve.
    ///
    /// # Returns
    ///
    /// `true` if the bit is set (1), and `false` if the bit is clear (0).
    /// If the index is out of bounds, it returns `false`.
    pub unsafe fn get(&self, index: usize) -> bool {
        // Check if the index is within the bounds of the bitmap. Return false if it's out of bounds.
        if index > (self.size * 8) {
            return false;
        }

        // Calculate the byte and bit position within the byte to find the target bit.
        let byte_index = index / 8; // Identify which byte the bit is in.
        let bit_index = index % 8; // Identify the bit's position within that byte.
                                   // Create a mask to isolate the target bit within the byte.
        let bit_indexer = 0b10000000 >> bit_index;

        // Apply the mask to the target byte and return whether the target bit is set or not.
        ((*self.buffer.add(byte_index as usize) as u8) & bit_indexer) != 0
    }

    /// Sets or clears a bit at a specific index in the bitmap.
    ///
    /// This function is unsafe because it directly modifies memory.
    /// It sets the bit at the specified index to the provided boolean value.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the bit to modify.
    /// * `value` - The boolean value to set the bit to. `true` sets the bit (1), and `false` clears the bit (0).
    ///
    /// # Returns
    ///
    /// `true` if the operation is within bounds and succeeds, `false` if the index is out of bounds.
    pub unsafe fn set(&mut self, index: usize, value: bool) -> bool {
        // Validate that the index is within the bitmap's bounds.
        if index > (self.size * 8) {
            return false;
        }

        // Determine the byte and bit position to modify.
        let byte_index = index / 8;
        let bit_index = index % 8;
        // Create a mask to manipulate the target bit.
        let bit_indexer = 0b10000000 >> bit_index;
        let ptr = self.buffer.add(byte_index as usize); // Get a pointer to the target byte.

        // Clear the bit position. If the value is true, set the bit; otherwise, the bit remains cleared.
        *ptr &= !bit_indexer; // Clear the target bit.
        if value {
            *ptr |= bit_indexer; // Set the target bit if the value is true.
        }
        true
    }
}
