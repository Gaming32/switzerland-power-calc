use std::fmt::{Display, Formatter};

include!(concat!(env!("OUT_DIR"), "/splat_lang.rs"));

impl Display for AnimationLanguage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.language_name())
    }
}

fn format_power_num(power: f64, include_negative_sign: bool) -> (String, String) {
    let scaled_power = (power * 10.0).floor().abs() as u32;
    let mut integer = full_width_number(scaled_power / 10);
    let fraction = full_width_number(scaled_power % 10);
    if include_negative_sign && power < 0.0 {
        integer.insert(0, '-');
    }
    (integer, fraction)
}

/// Encodes a number into fullwidth characters
fn full_width_number(mut x: u32) -> String {
    if x == 0 {
        return "０".to_string();
    }

    const BUF_SIZE: usize = (u32::MAX.ilog10() as usize + 1) * 3;
    let mut result = [0; BUF_SIZE];
    let mut offset = BUF_SIZE;
    while x > 0 {
        let digit = x % 10;
        x /= 10;

        result[offset - 3] = 239;
        result[offset - 2] = 188;
        result[offset - 1] = 144 + digit as u8;
        offset -= 3;
    }

    let result = &result[offset..];
    unsafe { str::from_utf8_unchecked(result) }.to_string()
}
