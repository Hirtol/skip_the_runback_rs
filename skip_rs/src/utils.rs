pub fn leak<T>(value: T) -> &'static mut T {
    Box::leak(Box::new(value))
}
