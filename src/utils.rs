//------------------------------------------------------------------------------
// MODULE: Convert
//------------------------------------------------------------------------------
pub mod convert {
    use std::str::FromStr;

    const THOUSANDS_SEP: char = ',';

    pub fn str_to_num<T: FromStr + Default>(s: &str) -> T {
        s.replace(THOUSANDS_SEP, "")
            .parse::<T>()
            .unwrap_or_default()
    }

    pub fn max_str<T: FromStr + Default + PartialOrd>(s1: &str, s2: &str) -> String {
        let n1 = str_to_num::<T>(s1);
        let n2 = str_to_num::<T>(s2);

        if n1 > n2 { s1 } else { s2 }.to_owned()
    }
}

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
