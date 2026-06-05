mod sys;

use crate::sys as imp;
use crate::sys::{IntoInner, FromInner, AsInner, AsInnerMut};


pub struct Pty {
    inner: imp::Pty,
}

impl IntoInner<imp::Pty> for Pty {
    fn into_inner(self) -> imp::Pty {
        self.inner
    }
}

impl FromInner<imp::Pty> for Pty {
    fn from_inner(inner: imp::Pty) -> Pty {
        Pty {
            inner: inner
        }
    }
}

impl AsInner<imp::Pty> for Pty {
    fn as_inner(&self) -> &imp::Pty {
        &self.inner
    }
}

impl AsInnerMut<imp::Pty> for Pty {
    fn as_inner_mut(&mut self) -> &mut imp::Pty {
        &mut self.inner
    }
}