#![allow(unused)]

/// Creates a macro that invokes another macro for each element.
/// Example:
/// ```rs
/// make_table!{ table_macro_name($doesnt_matter);
/// 	{ "{}", "Literally any tokens. Anything can go in here." }
/// 	{ "{}", 1234 }
/// }
/// table_macro_name!(println!);
/// ```
/// I would like to note: The `$doesnt_matter` portion is required.
/// macro_rules macros can't contain `$`, so you have to provide it
/// to the macro. The identifier doesn't really matter.
/// Each row in the table must be contained within a brace pair.
/// The contents of that block are fed into the macro that is fed
/// into the table macro.
/// That sounds a little confusing, so maybe I'll try to illustrate
/// it based on the above example.
/// ```rs
/// table_macro_name!(println!);
/// ```
/// Becomes
/// ```rs
/// println!("{}", "Literally any tokens. Anything can go in here.");
/// println!("{}", 1234);
/// ```
#[macro_export]
macro_rules! make_table {
	($name:ident($dolla:tt$rule:ident);$({$($item:tt)*})+) => {
		macro_rules! $name {
			($dolla$rule:path) => {
				$(
					$dolla$rule!{$($item)*}
				)*
			};
		}
	};
}

/// The purpose of this macro is to be able to generate code for each
/// primitive integer type (this means no f32 or f64).
/// You invoke the macro with the path to another macro that you would
/// like to invoke for each type.
/// Optionally you can restrict generation to either unsigned or signed
/// by typing `;unsigned` or `;signed` after the provided macro argument.
#[macro_export]
macro_rules! for_each_int_type {
	($macro:path) => {
		$crate::for_each_int_type!($macro;unsigned);
		$crate::for_each_int_type!($macro;signed);
	};
	($macro:path;unsigned) => {
		$macro!{usize}
		$macro!{u128}
		$macro!{u64}
		$macro!{u32}
		$macro!{u16}
		$macro!{u8}
	};
	($macro:path;signed) => {
		$macro!{isize}
		$macro!{i128}
		$macro!{i64}
		$macro!{i32}
		$macro!{i16}
		$macro!{i8}
	}
}

#[test]
fn print_types() {
	macro_rules! print_type {
		($token:tt) => {
			println!("{}", stringify!($token));
		};
	}
	for_each_int_type!(print_type);
}