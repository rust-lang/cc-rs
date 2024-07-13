use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::Once,
};

pub(crate) struct OnceCell<T> {
    once: Once,
    value: UnsafeCell<MaybeUninit<T>>,
    _marker: PhantomData<T>,
}

impl<T> OnceCell<T> {
    pub(crate) const fn new() -> Self {
        Self {
            once: Once::new(),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _marker: PhantomData,
        }
    }

    pub(crate) fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.once.call_once_force(|_| {
            unsafe { &mut *self.value.get() }.write(f());
        });
        unsafe { (&*self.value.get()).assume_init_ref() }
    }
}

unsafe impl<T: Sync + Send> Sync for OnceCell<T> {}
unsafe impl<T: Send> Send for OnceCell<T> {}

impl<T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for OnceCell<T> {}
impl<T: UnwindSafe> UnwindSafe for OnceCell<T> {}

impl<T> Drop for OnceCell<T> {
    #[inline]
    fn drop(&mut self) {
        if self.once.is_completed() {
            // SAFETY: The cell is initialized and being dropped, so it can't
            // be accessed again.
            unsafe { (&mut *self.value.get()).assume_init_drop() };
        }
    }
}
