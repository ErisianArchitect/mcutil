//! Extensions to Rust core stuff, like bool, numerics, Result, Option, etc.

pub trait ResultExtension<T,E> {
    // fn ok_err<R, OkFn: FnOnce(T) -> R, ErrFn: FnOnce(E) -> R>(self, ok: OkFn, err: ErrFn) -> R;
}

impl<T,E> ResultExtension<T,E> for Result<T,E> {
    // This seemed like a good idea, but there are borrowing issues.
    // Besides, Result already has mechanics to do this, you just
    // need to use `.map` and `.else` or whatever.
    // fn ok_err<R, OkFn: FnOnce(T) -> R, ErrFn: FnOnce(E) -> R>(self, ok: OkFn, err: ErrFn) -> R {
    // 	match self {
    // 		Ok(ok_value) => ok(ok_value),
    // 		Err(err_value) => err(err_value),
    // 	}
    // }
}

pub trait OptionExtension<T> {
    fn then<F: FnOnce(T)>(self, then: F);
}

impl<T> OptionExtension<T> for Option<T> {
    fn then<F: FnOnce(T)>(self, then: F) {
        if let Some(value) = self {
            then(value);
        }
    }
}

pub trait BoolExtension: 'static {
    fn choose<T>(self, _true: T, _false: T) -> T;
    fn invert(&mut self) -> Self;
    fn some<T>(self, some: T) -> Option<T>;
    fn some_else<T>(self, some: T) -> Option<T>;
    fn if_<F: Fn()>(self, _if: F);
    fn if_else<R, If: Fn() -> R, Else: Fn() -> R>(self, _if: If, _else: Else) -> R;
}

impl BoolExtension for bool {
    /// Choose a truth value or a false value.
    #[inline]
    fn choose<T>(self, _true: T, _false: T) -> T {
        if self {
            _true
        } else {
            _false
        }
    }

    /// Inverts the value of the boolean.
    #[inline]
    fn invert(&mut self) -> Self {
        if *self {
            *self = false;
        } else {
            *self = true;
        }
        *self
    }

    /// Returns `Some(some)` if true.
    #[inline]
    fn some<T>(self, some: T) -> Option<T> {
        self.choose(Some(some), None)
    }

    /// Returns `Some(some)` if false.
    #[inline]
    fn some_else<T>(self, some: T) -> Option<T> {
        self.choose(None, Some(some))
    }

    #[inline]
    fn if_<F: Fn()>(self, _if: F) {
        if self {
            _if();
        }
    }

    /// Like `if-else`, but with closures!
    #[inline]
    fn if_else<R, If: Fn() -> R, Else: Fn() -> R>(self, _if: If, _else: Else) -> R {
        if self {
            _if()
        } else {
            _else()
        }
    }
}

#[cfg(debug_assertions)]
mod tests {
    #[test]
    fn bool_test() {
        use super::*;
        let falsehood = true.invert();
        println!("Falsehood: {falsehood}");
        let text = false.some_else("Hello, world!");
        match text {
            Some(some) => println!("Some: {some}"),
            None => println!("None"),
        }
        print!("Value 1: ");
        let result = false.if_else(|| {
            println!("True");
            1
        }, || {
            println!("False");
            2
        });
        println!("Result: {result}");
        print!("Value 2: ");
        let result = true.if_else(|| {
            println!("True");
            1
        }, || {
            println!("False");
            2
        });
        println!("Result: {result}");
    }
}
