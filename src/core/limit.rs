pub trait Limit<T = Self> {
    const ASSERT_MESSAGE: &'static str;

    #[must_use]
    fn min() -> T;

    #[must_use]
    fn max() -> T;

    fn clamp(&mut self);

    #[must_use]
    fn clamped(mut self) -> Self
    where
        Self: Sized,
    {
        self.clamp();
        self
    }

    #[must_use]
    fn limit_check(&self) -> bool;

    #[must_use]
    fn limit_mut(&mut self) -> &mut Self {
        self.limit_assert();
        self
    }

    #[must_use]
    fn limit_ref(&self) -> &Self {
        self.limit_assert();
        self
    }

    #[must_use]
    fn limit_pass(self) -> Self
    where
        Self: Sized,
    {
        self.limit_assert();
        self
    }

    fn limit_assert(&self) {
        assert!(self.limit_check(), "{}", Self::ASSERT_MESSAGE);
    }

    fn limit_error<E>(&self, error: E) -> Result<(), E> {
        if self.limit_check() {
            Ok(())
        } else {
            Err(error)
        }
    }
}
