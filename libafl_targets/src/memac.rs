use libafl::{
    executors::{hooks::ExecutorHook, HasObservers},
    inputs::HasBytesVec,
};

use core::marker::PhantomData;
/// The hook to log the pointer the input buffer
pub struct MemacHook<I> {
    phantom: PhantomData<I>
}

extern "C" {
    /// where we log the pointer to the input buffer.
    pub static mut __data_pointer: *mut u8;
}

impl<I> ExecutorHook for MemacHook<I> 
where
    I: HasBytesVec,
{
    fn init<E, S>(&mut self, _: &mut S)
    where
        E: HasObservers,
    {
        unsafe {
            __data_pointer = core::ptr::null_mut();
        }
    }

    fn pre_exec<EM, I, S, Z>(&mut self, _: &mut Z, _: &mut S, _: &mut EM, input: &I) 
    where
    {
        self.remember_ptr(input);
    }
    fn post_exec<EM, I, S, Z>(&mut self, _: &mut Z, _: &mut S, _: &mut EM, _: &I) {}
}

impl<I> MemacHook<I> 
where
    I: HasBytesVec,
{
    fn remember_ptr(&self, input: &I)
    {
    }
}
