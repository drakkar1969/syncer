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
