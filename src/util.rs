macro_rules! make_enum((
    $(#[$meta:meta])* $vis:vis enum $Ty:ident;
    $($variant:ident => $s:literal),* $(,)?
) => {
    $(#[$meta])*
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    $vis enum $Ty {
        $(#[doc = concat!("`", $s, "`")] $variant,)*
    }

    impl $Ty {
        /// Converts it to a string.
        pub fn as_str(self) -> &'static str {
            match self {
                $($Ty::$variant => $s,)*
            }
        }

        /// Converts it from a string.
        pub fn from_str(s: &str) -> Option<Self> {
            match s {
                $($s => Some($Ty::$variant),)*
                _ => None,
            }
        }
    }

    impl std::fmt::Display for $Ty {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.as_str())
        }
    }
});
pub(crate) use make_enum;
