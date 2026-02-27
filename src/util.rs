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

macro_rules! make_id((
    $(#[$meta:meta])* $vis:vis struct $Ty:ident(..);
    $(iter $Iter:ident $plural:literal;)?
) => {
    $(#[$meta])*
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    $vis struct $Ty(u32);

    impl $Ty {
        /// Gets the index of the ID.
        $vis fn as_usize(self) -> usize {
            self.0 as usize
        }
    }

    $(#[doc = concat!("Iterator over ", $plural, " in a half-open range.")]
    #[derive(Clone, Debug)]
    pub struct $Iter {
        front: u32,
        back: u32,
    }

    impl TermId {
        #[doc = concat!("Creates an iterator over ", $plural, " in a half-open range.")]
        pub fn iter(range: Range<TermId>) -> $Iter {
            $Iter {
                front: range.start.0,
                back: range.end.0.max(range.start.0),
            }
        }
    }

    impl Iterator for $Iter {
        type Item = TermId;

        fn next(&mut self) -> Option<Self::Item> {
            if self.front == self.back {
                return None;
            }
            let id = TermId(self.front);
            self.front += 1;
            Some(id)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = (self.back - self.front) as usize;
            (len, Some(len))
        }
    }

    impl DoubleEndedIterator for $Iter {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.front == self.back {
                return None;
            }
            self.back -= 1;
            Some(TermId(self.back))
        }
    }

    impl ExactSizeIterator for $Iter {})?
});
pub(crate) use make_id;
