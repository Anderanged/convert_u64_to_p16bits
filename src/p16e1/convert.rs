use super::P16E1;
use crate::u32_with_sign;
use crate::u64_with_sign;
use core::{f32, f64};

crate::macros::impl_convert!(P16E1);

impl P16E1 {
    #[inline]
    pub const fn from_i32(mut i_a: i32) -> Self {
        if i_a < -134_217_728 {
            //-2147483648 to -134217729 rounds to P32 value -268435456
            return Self::MIN;
        }
        let sign = i_a.is_negative();
        if sign {
            i_a = -i_a;
        }
        Self::from_bits(convert_u32_to_p16bits(i_a as u32)).with_sign(sign)
    }

    #[inline]
    pub const fn from_u32(a: u32) -> Self {
        Self::from_bits(convert_u32_to_p16bits(a))
    }

    #[inline]
    pub const fn from_i64(mut i_a: i64) -> Self {
        if i_a < -134_217_728 {
            //-2147483648 to -134217729 rounds to P32 value -268435456
            return Self::MIN;
        }
        let sign = i_a.is_negative();
        if sign {
            i_a = -i_a;
        }
        Self::from_bits(convert_u64_to_p16bits(i_a as u64)).with_sign(sign)
    }

    #[inline]
    pub const fn from_u64(a: u64) -> Self {
        Self::from_bits(convert_u64_to_p16bits(a))
    }

    #[inline]
    pub const fn to_i32(self) -> i32 {
        let mut ui_a = self.to_bits(); // Copy of the input.

        if ui_a == 0x8000 {
            return 0;
        }

        let sign = ui_a > 0x8000; // sign is True if pA > NaR.
        if sign {
            ui_a = ui_a.wrapping_neg(); // A is now |A|.
        }
        let i_z = convert_p16bits_to_u32(ui_a);

        u32_with_sign(i_z, sign) as i32
    }

    #[inline]
    pub const fn to_u32(self) -> u32 {
        let ui_a = self.to_bits(); // Copy of the input.

        if ui_a >= 0x8000 {
            return 0; //negative
        }
        convert_p16bits_to_u32(ui_a)
    }

    #[inline]
    pub const fn to_i64(self) -> i64 {
        let mut ui_a = self.to_bits();

        // NaR
        if ui_a == 0x8000 {
            return 0;
        }

        let sign = (ui_a & 0x_8000) != 0;
        if sign {
            ui_a = ui_a.wrapping_neg();
        }

        let i_z = convert_p16bits_to_u64(ui_a);

        u64_with_sign(i_z, sign) as i64
    }

    #[inline]
    pub const fn to_u64(self) -> u64 {
        let ui_a = self.to_bits();

        if ui_a >= 0x8000 {
            return 0;
        }
        convert_p16bits_to_u64(ui_a)
    }
}

const fn convert_u32_to_p16bits(a: u32) -> u16 {
    if a > 0x0800_0000 {
        0x7FFF
    } else if a > 0x02FF_FFFF {
        0x7FFE
    } else if a < 2 {
        (a as u16) << 14
    } else {
        let mut frac_a = a;
        let mask = 0x0200_0000_u32;
        let mut log2 = 25_i8;
        while (frac_a & mask) == 0 {
            log2 -= 1;
            frac_a <<= 1;
        }
        let k: i8 = log2 >> 1;
        let exp_a: u32 = ((log2 & 0x1) as u32) << (12 - k);
        frac_a ^= mask;

        let mut ui_a = ((0x7FFF ^ (0x3FFF >> k)) | exp_a | (frac_a >> (k + 13))) as u16;
        let mask = 0x1000 << k; //bit_n_plus_one
        if ((mask & frac_a) != 0) && ((((mask - 1) & frac_a) | ((mask << 1) & frac_a)) != 0) {
            ui_a += 1;
        }
        ui_a
    }
}

const fn convert_u64_to_p16bits(a: u64) -> u16 {
    if a > 0x0000_0000_0800_0000 {
        0x7FFF
    } else if a > 0x0000_0000_02FF_FFFF {
        0x7FFE
    } else if a < 2 {
        (a as u16) << 14
    } else {
        let mut mask = 0x0000_0000_0200_0000_u64;
        let mut frac_a = a;
        let mut log2 = 25_i8;
        while (frac_a & mask) == 0 {
            log2 -= 1;
            frac_a <<= 1;
        }
        let k = log2 >> 1;
        let exp_a = ((log2 & 0x1) as u64) << (12 - k);
        frac_a ^= mask;
        let mut ui_a = ((0x7FFF ^ (0x3FFF >> k)) | exp_a | (frac_a >> (k + 13))) as u16;
        mask = 0x1000 << k;
        if ((mask & frac_a) != 0) && ((((mask - 1) & frac_a) | ((mask << 1) & frac_a)) != 0) {
            ui_a += 1;
        }
        ui_a
    }
}

const fn convert_p16bits_to_u32(ui_a: u16) -> u32 {
    if ui_a <= 0x3000 {
        // 0 <= |pA| <= 1/2 rounds to zero.
        0
    } else if ui_a < 0x4800 {
        // 1/2 < x < 3/2 rounds to 1.
        1
    } else if ui_a <= 0x5400 {
        // 3/2 <= x <= 5/2 rounds to 2.
        2
    } else {
        let (scale, bits) = P16E1::calculate_scale(ui_a);

        let mut i_z = ((bits as u32) | 0x2000) << 17; // Left-justify fraction in 32-bit result (one left bit padding)
        let mut mask = 0x4000_0000_u32 >> scale; // Point to the last bit of the integer part.

        let bit_last = (i_z & mask) != 0; // Extract the bit, without shifting it.
        mask >>= 1;
        let mut tmp = i_z & mask;
        let bit_n_plus_one = tmp != 0; // "True" if nonzero.
        i_z ^= tmp; // Erase the bit, if it was set.
        tmp = i_z & (mask - 1); // tmp has any remaining bits. // This is bits_more
        i_z ^= tmp; // Erase those bits, if any were set.

        if bit_n_plus_one {
            // logic for round to nearest, tie to even
            if ((bit_last as u32) | tmp) != 0 {
                i_z += mask << 1;
            }
        }
        i_z >> (30 - scale) // Right-justify the integer.
    }
}

const fn convert_p16bits_to_u64(ui_a: u16) -> u64 {
    if ui_a <= 0x3000 {
        0
    } else if ui_a < 0x4800 {
        1
    } else if ui_a <= 0x5400 {
        2
    } else {
        let (scale, bits) = P16E1::calculate_scale(ui_a);

        let mut i_z = ((bits as u64) | 0x2000) << 49;

        let mut mask = 0x4000_0000_0000_0000_u64 >> scale;

        let bit_last = (i_z & mask) != 0;
        mask >>= 1;
        let mut tmp = i_z & mask;
        let bit_n_plus_one = tmp != 0;
        i_z ^= tmp;
        tmp = i_z & (mask - 1); // bits_more
        i_z ^= tmp;

        if bit_n_plus_one && (((bit_last as u64) | tmp) != 0) {
            i_z += mask << 1;
        }
        i_z >> (62 - scale)
    }
}

impl P16E1 {
    pub const fn from_f32(float: f32) -> Self {
        use crate::RawFloat;
        let ui: u32 = f32::to_bits(float);

        let sign = (ui & f32::SIGN_MASK) != 0;

        let uip = ui & !f32::SIGN_MASK;
        // check zero
        if uip == 0 {
            Self::ZERO
        } else if uip >= 0x_7f80_0000 {
            Self::NAR
        } else if uip >= 0x_4d80_0000 {
            // +- 268_435_456.
            if !sign {
                Self::MAX
            } else {
                Self::MIN
            }
        } else if uip == 0x_3f80_0000 {
            // +- 1.
            if !sign {
                Self::ONE
            } else {
                Self::ONE.neg()
            }
        } else if uip <= 0x_3180_0000 {
            // +- 3.725_290_298_461_914_e-9
            if !sign {
                Self::MIN_POSITIVE
            } else {
                Self::MIN_POSITIVE.neg()
            }
        } else {
            Self::from_bits(crate::convert::convert_float!(P16E1, f32, ui))
        }
    }

    pub const fn from_f64(float: f64) -> Self {
        use crate::RawFloat;
        let ui: u64 = f64::to_bits(float);

        let sign = (ui & f64::SIGN_MASK) != 0;

        let uip = ui & !f64::SIGN_MASK;
        // check zero
        if uip == 0 {
            Self::ZERO
        } else if uip >= 0x_7ff0_0000_0000_0000 {
            Self::NAR
        } else if uip >= 0x_41b0_0000_0000_0000 {
            // +- 268_435_456.
            if !sign {
                Self::MAX
            } else {
                Self::MIN
            }
        } else if uip == 0x_3ff0_0000_0000_0000 {
            // +- 1.
            if !sign {
                Self::ONE
            } else {
                Self::ONE.neg()
            }
        } else if uip <= 0x_3e30_0000_0000_0000 {
            // +- 3.725_290_298_461_914_e-9
            if !sign {
                Self::MIN_POSITIVE
            } else {
                Self::MIN_POSITIVE.neg()
            }
        } else {
            Self::from_bits(crate::convert::convert_float!(P16E1, f64, ui))
        }
    }

    #[inline]
    pub const fn to_f32(self) -> f32 {
        let mut ui_a = self.to_bits();

        if self.is_zero() {
            0.
        } else if self.is_nar() {
            f32::NAN
        } else {
            let sign_a = ui_a & P16E1::SIGN_MASK;
            if sign_a != 0 {
                ui_a = ui_a.wrapping_neg();
            }
            let (k_a, tmp) = P16E1::separate_bits_tmp(ui_a);

            let frac_a = ((tmp << 2) as u32) << 7;
            let exp_a = (((k_a as u32) << 1) + ((tmp >> 14) as u32)).wrapping_add(127) << 23;

            f32::from_bits(exp_a + frac_a + ((sign_a as u32) << 16))
        }
    }
}

impl P16E1 {
    #[inline]
    pub const fn to_f64(self) -> f64 {
        let mut ui_a = self.to_bits();

        if self.is_zero() {
            0.
        } else if self.is_nar() {
            f64::NAN
        } else {
            let sign_a = ui_a & P16E1::SIGN_MASK;
            if sign_a != 0 {
                ui_a = ui_a.wrapping_neg();
            }
            let (k_a, tmp) = P16E1::separate_bits_tmp(ui_a);

            let frac_a = ((tmp << 2) as u64) << 36;
            let exp_a = (((k_a as u64) << 1) + ((tmp >> 14) as u64)).wrapping_add(1023) << 52;

            f64::from_bits(exp_a + frac_a + ((sign_a as u64) << 48))
        }
    }
}

#[test]
fn convert_p16_f64() {
    for n in -0x_8000_i16..0x_7fff {
        let p = P16E1::new(n);
        let f = f64::from(p);
        assert_eq!(p, P16E1::from(f));
    }
}

#[test]
fn convert_p16_f32() {
    for n in -0x_8000_i16..0x_7fff {
        let p = P16E1::new(n);
        let f = f32::from(p);
        assert_eq!(p, P16E1::from(f));
    }
}

#[test]
fn convert_f64_p8_rand() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..crate::NTESTS16 {
        let f: f64 = rng.gen();
        let _p = P16E1::from(f);
    }
}

#[test]
fn convert_f32_p16_rand() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..crate::NTESTS16 {
        let f: f32 = rng.gen();
        let _p = P16E1::from(f);
    }
}

#[test]
fn convert_p16_i32() {
    for n in -0x_8000_i16..0x_7fff {
        let p = P16E1::new(n);
        let f = f64::from(p).round();
        if p % P16E1::new(0x_3000) == P16E1::ZERO {
            continue;
        }
        assert_eq!(i32::from(p), f as i32);
    }
}

#[test]
fn convert_p16_i64() {
    for n in -0x_8000_i16..0x_7fff {
        let p = P16E1::new(n);
        let f = f64::from(p).round();
        if p % P16E1::new(0x_3000) == P16E1::ZERO {
            continue;
        }
        assert_eq!(i64::from(p), f as i64);
    }
}

#[test]
fn convert_p16bits_u64() {
    for n in 0x5401..0x7fffu16 {
        let u = convert_p16bits_to_u64(n);
        // range here is huge but honestly it's simple and should work.
        // would like something more
        assert!(u > 2);
        assert!(u < 0x1000_0000);
    }
}

#[test]
fn convert_p16bits_u32() {
    for n in 0x5401..0x7fffu16 {
        let u = convert_p16bits_to_u32(n);
        assert!(u > 2);
        assert!(u < 0x1000_0000);
    }
}

#[test]
fn convert_u64_p16bits() {
    assert_eq!(P16E1::ZERO.to_bits(), convert_u64_to_p16bits(0));
    assert_eq!(P16E1::ONE.to_bits(), convert_u64_to_p16bits(1));
    assert_eq!(P16E1::MAX.to_bits(), convert_u64_to_p16bits(0x1000_0000));
    // more rigorous method needed to truly test all possible numbers.
    // however, this should suffice as a warning.
    assert_eq!(0b0_1110_1_00_0000_0000u16, convert_u64_to_p16bits(32));
    assert_eq!(0b0_110_1_111_0000_0000u16, convert_u64_to_p16bits(15));
}

#[test]
fn convert_u32_p16bits() {
    assert_eq!(P16E1::ZERO.to_bits(), convert_u32_to_p16bits(0));
    assert_eq!(P16E1::ONE.to_bits(), convert_u32_to_p16bits(1));
    assert_eq!(P16E1::MAX.to_bits(), convert_u32_to_p16bits(0x1000_0000));
    assert_eq!(0b0_10_1_1000_0000_0000u16, convert_u32_to_p16bits(3));
    assert_eq!(0b0_11110_1_0_0000_0000u16, convert_u32_to_p16bits(128));
}
