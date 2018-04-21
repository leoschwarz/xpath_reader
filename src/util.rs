use std::borrow::Borrow;

// TODO: Is there a standard type for this in Rust, like Cow but without
//       the clone requirement.
#[derive(Debug)]
pub(crate) enum Refable<'a, T: 'a> {
    Owned(T),
    Borrowed(&'a T),
}

impl<'a, T> Borrow<T> for Refable<'a, T> {
    fn borrow(&self) -> &T {
        match self {
            &Refable::Owned(ref v) => &v,
            &Refable::Borrowed(v) => v,
        }
    }
}

impl<'a, T> Refable<'a, T> {
    pub fn clone_ref(&'a self) -> Self {
        Refable::Borrowed(self.borrow())
    }
}
