pub struct Scale<T, Src = (), Dst = ()> {
    pub x: T,
    pub y: T,
    _unit: std::marker::PhantomData<(Src, Dst)>,
}

impl<T, Src, Dst> Scale<T, Src, Dst> {
    #[must_use]
    pub const fn new(x: T, y: T) -> Self {
        Self {
            x,
            y,
            _unit: std::marker::PhantomData,
        }
    }
}

impl<T: Clone, Src, Dst> Clone for Scale<T, Src, Dst> {
    fn clone(&self) -> Self {
        Self::new(self.x.clone(), self.y.clone())
    }
}

impl<T: Copy, Src, Dst> Copy for Scale<T, Src, Dst> {}

impl<T: Default, Src, Dst> Default for Scale<T, Src, Dst> {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl<T: std::fmt::Debug, Src, Dst> std::fmt::Debug for Scale<T, Src, Dst> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("from", &std::any::type_name::<Src>())
            .field("to", &std::any::type_name::<Dst>())
            .finish()
    }
}
