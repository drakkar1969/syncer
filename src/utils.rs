//------------------------------------------------------------------------------
// MODULE: Case
//------------------------------------------------------------------------------
pub mod case {
    pub fn capitalize_first(s: &str) -> String {
        let mut s = s.to_owned();

        if let Some(first) = s.get_mut(0..1) {
            first.make_ascii_uppercase();
        }

        s
    }
}

//------------------------------------------------------------------------------
// MODULE: Convert
//------------------------------------------------------------------------------
pub mod convert {
    use std::sync::OnceLock;
    use std::str::FromStr;

    use num_format::{SystemLocale, ToFormattedString};

    fn sys_locale() -> &'static SystemLocale {
        static LOCALE: OnceLock<SystemLocale> = OnceLock::new();
        LOCALE.get_or_init(|| {
            SystemLocale::default().unwrap()
        })
    }

    pub fn string_to_num<T: FromStr + Default>(s: &str) -> T {
        s.replace(sys_locale().separator(), "")
            .parse::<T>()
            .unwrap_or_default()
    }

    pub fn num_to_string<T: ToFormattedString>(i: T) -> String {
        i.to_formatted_string(sys_locale())
    }
}

//------------------------------------------------------------------------------
// MODULE: Size
//------------------------------------------------------------------------------
pub mod size {
    pub fn format(size: &str) -> String {
        let (num, unit) = size.find(|ch: char| ch.is_alphabetic())
            .map_or_else(|| (size, ""), |i| size.split_at(i));

        format!("{num}\u{202F}{unit}")
    }
}
