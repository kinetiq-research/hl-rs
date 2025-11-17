use crate::{Error, Result};

pub fn float_to_int(x: f64, power: u32) -> Result<i64> {
    let with_decimals = x * 10_f64.powi(power as i32);
    let rounded = with_decimals.round();

    if (rounded - with_decimals).abs() >= 1e-3 {
        return Err(Error::GenericParse(format!(
            "float_to_int causes rounding: {}",
            x
        )));
    }

    Ok(rounded as i64)
}

pub fn float_to_int_for_hashing(x: f64) -> Result<i64> {
    float_to_int(x, 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_to_int_for_hashing() -> Result<()> {
        assert_eq!(float_to_int_for_hashing(92233720.0)?, 9223372000000000);
        assert_eq!(float_to_int_for_hashing(0.00001231)?, 1231);
        assert_eq!(float_to_int_for_hashing(1.033)?, 103300000);
        assert_eq!(float_to_int_for_hashing(100.0)?, 10000000000);
        Ok(())
    }
}
