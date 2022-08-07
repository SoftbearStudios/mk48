pub fn is_default<T: Default + PartialEq>(x: &T) -> bool {
    x == &T::default()
}
