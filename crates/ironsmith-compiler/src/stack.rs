//! Native stack growth with a wasm no-op boundary.
//!
//! Browser runtimes provide a fixed wasm stack and `stacker` pulls native
//! object/archive tooling into wasm builds without being able to grow it.

#[cfg(not(target_arch = "wasm32"))]
pub fn maybe_grow<R>(red_zone: usize, stack_size: usize, callback: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(red_zone, stack_size, callback)
}

#[cfg(target_arch = "wasm32")]
pub fn maybe_grow<R>(_red_zone: usize, _stack_size: usize, callback: impl FnOnce() -> R) -> R {
    callback()
}
