use core::marker::PhantomData;

use libafl::{
    executors::{hooks::ExecutorHook, HasObservers},
    inputs::{HasBytesVec, UsesInput},
};
/// The hook to log the pointer the input buffer
#[derive(Debug)]
pub struct MemacHook<S>
where
    S: UsesInput,
{
    phantom: PhantomData<S>,
}

impl<S> Default for MemacHook<S>
where
    S: UsesInput,
    S::Input: HasBytesVec,
{
    fn default() -> Self {
        Self::new()
    }
}

extern "C" {
    /// where we log the pointer to the input buffer.
    pub static mut __input_start: *mut u8;
    /// where we log the pointer to the end of the input buffer.
    pub static mut __input_end: *mut u8;
}

impl<S> ExecutorHook<S> for MemacHook<S>
where
    S: UsesInput,
    S::Input: HasBytesVec,
{
    fn init<E: HasObservers>(&mut self, _: &mut S) {
        unsafe {
            __input_start = core::ptr::null_mut();
        }
    }

    fn pre_exec(&mut self, _: &mut S, input: &S::Input) {
        let ptr = input.bytes().as_ptr() as u64 as *mut u8;
        let len = input.bytes().len();
        unsafe {
            __input_start = ptr;
            __input_end = ptr.add(len);
        }
    }
    fn post_exec(&mut self, _: &mut S, _: &S::Input) {}
}

impl<S> MemacHook<S>
where
    S: UsesInput,
    S::Input: HasBytesVec,
{
    /// Constructor for this hook
    #[must_use]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
