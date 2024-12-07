#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::stdout::print(format_args!($($arg)*));
    };
}
