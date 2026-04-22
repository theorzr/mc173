//! Fluid block special functions (mostly for water).


/// Return true if this still/moving fluid block acts like a source.
#[inline]
pub fn is_source(metadata: u8) -> bool {
    metadata == 0
}

/// Force this metadata to be a source fluid block. This basically just overwrite metadata
/// with a 0, which means that distance is 0 and fluid is not falling.
#[inline]
pub fn set_source(metadata: &mut u8) {
    *metadata = 0;
}

/// Get the distance to source of a fluid block. The distance can go up to 7, but does
/// not account for the falling state.
#[inline]
pub fn get_distance(metadata: u8) -> u8 {
    metadata & 7
}

#[inline]
pub fn set_distance(metadata: &mut u8, distance: u8) {
    debug_assert!(distance <= 7);
    *metadata &= !7;
    *metadata |= distance;
} 

/// Get if this fluid block is falling and therefore should not spread on sides.
#[inline]
pub fn is_falling(metadata: u8) -> bool {
    metadata & 8 != 0
}

#[inline]
pub fn set_falling(metadata: &mut u8, falling: bool) {
    *metadata &= !8;
    *metadata |= (falling as u8) << 3;
}

/// This function get the actual distance to the source of a fluid block, this account 
/// both the distance stored in the lower 3 bits, but also for the falling state: if a
/// fluid is falling, it acts like a source block for propagation.
#[inline]
pub fn get_actual_distance(metadata: u8) -> u8 {
    if is_falling(metadata) {
        0
    } else {
        get_distance(metadata)
    }
}

/// Calculate the actual height of a fluid block depending on its metadata. This is the
/// height of the fluid's bounding box, for example a source block takes 14 pixels out
/// of the 16 of a block, in height, so 0.125 of a block, so the full block is 0.875. 
/// 
/// The actual original Minecraft code is quite weird, because it split the block in 
/// 9 different vertical chunks, and the last one is never filled, so for source blocks,
/// they are actually `8/9 = 0.88...` blocks tall, which is a bit more that the pixel 
/// size.
#[inline]
pub fn get_actual_height(metadata: u8) -> f32 {
    (7 - get_actual_distance(metadata) + 1) as f32 / 9.0
}

/// Calculate the height of a fluid block depending on its metadata, this is different
/// than [`get_actual_height`] in that it make falling and source blocks having a height 
/// of 1, instead of `8/9`. Plus the flowing fluids are also divided in 8, instead of 9,
/// this a distance of 7 (the maximum), will have 1/8, and not 1/9.
pub fn get_full_height(metadata: u8) -> f32 {
    (7 - get_actual_distance(metadata) + 1) as f32 / 8.0
}
