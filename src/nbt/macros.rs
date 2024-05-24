/// Shorthand way to create a Tag::Compound.
/// Example:
/// ```no_run
/// compound!{
///     ("Item One", 0i8),
///     (String::from("Item Two"), 2i32),
///     ("Item Three", Tag::Byte(1))
/// }
/// ```
#[macro_export]
macro_rules! compound {
    ($(($name:expr, $value:expr)),+$(,)?) => {
        $crate::nbt::tag::Tag::Compound($crate::nbt::Map::from([
            $(
                ($crate::list!(@literal_to_owned;$name), $crate::nbt::tag::Tag::from($value)),
            )+
        ]))
    };
    // ($($name:expr => $value:expr),+$(,)?) => {

    // };
    () => {
        $crate::nbt::tag::Tag::Compound($crate::Map::new())
    };
}

/// Shorthand way to create a Tag::List.
/// Example:
/// ```no_run
/// list!{ 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 };
/// list![
///     "One",
///     "Two",
///     "Three",
///     "Four",
///     "Five"
/// ];
/// ```
#[macro_export]
macro_rules! list {
    ($($item:expr),+$(,)?) => {
        $crate::tag::Tag::List($crate::tag::ListTag::from(std::vec![
            $(
                $crate::list!(@literal_to_owned;$item),
            )+
        ]))
    };
    ($value:expr; $repititions:expr) => {
        $crate::tag::Tag::List($crate::tag::ListTag::from(std::vec![$crate::list!(@literal_to_owned;$value); $repititions]))
    };
    () => {
        $crate::tag::Tag::List($crate::tag::ListTag::Empty);
    };
    (@literal_to_owned;$lit:literal) => {
        $lit.to_owned()
    };
    (@literal_to_owned;$($other:tt)+) => {
        $($other)+
    };
}

pub use list;
pub use compound;

#[cfg(test)]
mod tests {
    #[test]
    fn compound_test() {
        let tag = compound! {
            ("Hello, world.", "The quick brown fox jumps over the lazy dog."),
            ("Hello, world.", "The quick brown fox jumps over the lazy dog.")
        };
    }
}