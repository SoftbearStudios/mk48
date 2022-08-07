use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

/// Like [`Rc`] but always implements [`Eq`] and [`PartialEq`] according to reference equality.
#[repr(transparent)]
#[derive(Default)]
pub struct PtrEqRc<T>(Rc<T>);

impl<T> PtrEqRc<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(value))
    }
}

impl<T> Clone for PtrEqRc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Debug> Debug for PtrEqRc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Deref for PtrEqRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> PartialEq for PtrEqRc<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for PtrEqRc<T> {}
