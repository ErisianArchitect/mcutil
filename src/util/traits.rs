
/// The purpose of this trait is to tranform a type from itself into another type.
/// It's for type coercion in places where type coercion is necesasary, such as with Iterators.
/// ```rs
/// pub fn mutate_iter<I: TypeTransform<MyType>, It: IntoIterator<Item = I>>(items: It) -> Vec<MyType> {
/// 	let items = items.into_iter()
/// 		// Mutate the iterator using I::transform
/// 		.map(I::transform)
/// 		.collect::<Vec<MyType>>()
/// }
/// ```
pub trait TypeTransform<R> {
	fn transform(self) -> R;
}

impl<T, R: From<T>> TypeTransform<R> for T {
	fn transform(self) -> R {
		R::from(self)
	}
}