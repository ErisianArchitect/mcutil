
pub trait TypeTransform<R> {
	fn transform(self) -> R;
}

impl<T, R: From<T>> TypeTransform<R> for T {
	fn transform(self) -> R {
		R::from(self)
	}
}