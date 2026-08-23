#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn maybe_grow<R>(red_zone: usize, stack_size: usize, callback: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(red_zone, stack_size, callback)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn maybe_grow<R>(
    _red_zone: usize,
    _stack_size: usize,
    callback: impl FnOnce() -> R,
) -> R {
    callback()
}
