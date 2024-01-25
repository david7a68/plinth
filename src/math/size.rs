pub struct Size<T, U> {
    pub width: T,
    pub height: T,
    _unit: std::marker::PhantomData<U>,
}

impl<T, U> Size<T, U> {
    #[must_use]
    pub const fn new(width: T, height: T) -> Self {
        Self {
            width,
            height,
            _unit: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn retype<U2>(self) -> Size<T, U2> {
        Size::new(self.width, self.height)
    }
}

impl<T: Clone, U> Clone for Size<T, U> {
    fn clone(&self) -> Self {
        Self {
            width: self.width.clone(),
            height: self.height.clone(),
            _unit: std::marker::PhantomData,
        }
    }
}

impl<T: Copy, U> Copy for Size<T, U> {}

impl<T: Default, U> Default for Size<T, U> {
    fn default() -> Self {
        Self::new(T::default(), T::default())
    }
}

impl<T: std::fmt::Debug, U> std::fmt::Debug for Size<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Size")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("in", &std::any::type_name::<U>())
            .finish()
    }
}

impl<T: std::fmt::Debug, U> std::fmt::Display for Size<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl<T: PartialEq, U> PartialEq for Size<T, U> {
    fn eq(&self, rhs: &Self) -> bool {
        self.width == rhs.width && self.height == rhs.height
    }
}

impl<T: Eq, U> Eq for Size<T, U> {}
