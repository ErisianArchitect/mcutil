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

/// Measures the execution time of some set of instructions.
#[macro_export]
macro_rules! measure_time {
    ($($token:stmt)*) => {
        {
            let now = std::time::Instant::now();
            $($token)*
            now.elapsed()
        }
    };
}

#[test]
fn timetest() {
    let time = measure_time!{
        std::thread::sleep(std::time::Duration::from_secs(1));
        #[derive(Debug)]
        struct A {
            name: String,
        }
        let a = A { name: "Test".into() };
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

/// Continue a loop if a condition is met.
/// ```rs
/// let mut index = 0;
/// loop {
/// 	continue_if!((index & 1) == 0);
/// 	println!("{}", index);
/// 	index += 1;
/// 	if index > 10 {
/// 		break;
/// 	}
/// }
/// ```
/// Alternatively, you can also use a loop identifier:
/// 'x: for x in 0..32 {
/// 	'y: for y in 0..32 {
/// 		continue_if!('y: (y & 1) == 1)
/// 		continue_if!('x: y == 10);
/// 	}
/// }
#[macro_export]	
macro_rules! continue_if {
    ($($label:lifetime : )? $condition:expr) => {
        if $condition { continue $($label)?; }
    };
}

/// Break from a loop if a condition is met.
/// ```rs
/// let mut index = 0;
/// loop {
/// 	println!("{}", index);
/// 	index += 1;
/// 	break_if!(index >= 10);
/// }
/// ```
/// Alternatively, you can also use a loop identifier:
/// ```rs
/// 'x: for x in 0..32 {
/// 	'y: for y in 0..32 {
/// 		break_if!('y: x + y > 40);
/// 	}
/// }
/// ```
/// And lastly, you can also include a return value:
/// ```rs
/// let mut i = 0;
/// let result = loop {
/// 	break_if!(i == 10 => 10);
/// 	i += 1;
/// };
/// println!("Result: {result}");
/// ```
#[macro_export]	
macro_rules! break_if {
    ($($label:lifetime:)? $condition:expr $(=> $result:expr)?) => {
        if $condition { break $($label)? $($result)?; }
    };
}

/// Return from a function if a condition is met.
/// ```rs
/// let mut index = 0;
/// loop {
/// 	println!("{}", index);
/// 	index += 1;
/// 	return_if!(index >= 10);
/// }
/// ```
/// Alternatively, you can also provide an expression to be returned:
/// ```rs
/// // return_if!(condition => expr)
/// fn sample() -> (i32, i32) {
/// 	for x in 0..32 {
/// 		for y in 0..32 {
/// 			return_if!(x + y == 40 => (x, y));
/// 		}
/// 	}
/// 	(0, 0)
/// }
/// ```
#[macro_export]
macro_rules! return_if {
    ($condition:expr $(=> $result:expr)?) => {
        if $condition {
            return $($result)?;
        }
    };
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