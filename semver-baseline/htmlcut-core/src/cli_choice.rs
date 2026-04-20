/// Stable string-backed choice domain exposed on HTMLCut's CLI surface.
pub trait CliChoice: Copy + Eq + 'static {
    /// Returns the full stable set of accepted CLI values.
    fn variants() -> &'static [Self];

    /// Returns the stable user-facing spelling for one choice.
    fn as_cli_str(self) -> &'static str;

    /// Parses one stable CLI spelling into this choice domain.
    fn parse_cli_str(value: &str) -> Option<Self> {
        Self::variants()
            .iter()
            .copied()
            .find(|variant| variant.as_cli_str() == value)
    }
}

macro_rules! impl_cli_choice {
    ($ty:ty { $($variant:path => $name:literal),+ $(,)? }) => {
        impl $crate::CliChoice for $ty {
            fn variants() -> &'static [Self] {
                const VARIANTS: &[$ty] = &[$($variant),+];
                VARIANTS
            }

            fn as_cli_str(self) -> &'static str {
                match self {
                    $(
                        $variant => $name,
                    )+
                }
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(<Self as $crate::CliChoice>::as_cli_str(*self))
            }
        }
    };
}

pub(crate) use impl_cli_choice;
