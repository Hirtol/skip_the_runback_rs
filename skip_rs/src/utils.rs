/// Horrifically unsafe, mainly here to get around the fact that all `frida_gum` types are `!Send + !Sync` due to the raw
/// pointers embedded in their structs.
pub struct NullLock<T>(pub T);

impl<T> std::ops::Deref for NullLock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for NullLock<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<T> Sync for NullLock<T> {}
unsafe impl<T> Send for NullLock<T> {}
