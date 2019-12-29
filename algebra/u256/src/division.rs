use crate::{
    algorithms::{divrem_nby1, divrem_nbym, inv_mod, limb_operations::div_2_1},
    noncommutative_binop, Binary, DivRem, InvMod, U256,
};
use num_traits::Inv;
use std::{
    num::Wrapping,
    ops::{Div, DivAssign, Rem, RemAssign},
    prelude::v1::*,
    u64,
};

// Division like routines: Integer division/remaindering, Ring
// division/inversion Modular inversions/divisions.

impl InvMod for U256 {
    /// Computes the inverse modulo a given modulus
    #[inline(always)]
    fn inv_mod(&self, modulus: &Self) -> Option<Self> {
        inv_mod(modulus, self)
    }
}

impl DivRem<u64> for &U256 {
    type Quotient = U256;
    type Remainder = u64;

    // Short division
    // TODO: Can be computed in-place
    fn div_rem(self, rhs: u64) -> Option<(U256, u64)> {
        if rhs == 0 {
            None
        } else {
            // Knuth Algorithm S
            // 4 by 1 division
            let (q3, r) = div_2_1(self.limb(3), 0, rhs);
            let (q2, r) = div_2_1(self.limb(2), r, rhs);
            let (q1, r) = div_2_1(self.limb(1), r, rhs);
            let (q0, r) = div_2_1(self.limb(0), r, rhs);
            Some((U256::from_limbs([q0, q1, q2, q3]), r))
        }
    }
}

impl<'a> DivRem<&'a U256> for &'a U256 {
    type Quotient = U256;
    type Remainder = U256;

    // Short division
    // TODO: Can be computed in-place
    fn div_rem(self, rhs: Self) -> Option<(U256, U256)> {
        let mut numerator = [self.limb(0), self.limb(1), self.limb(2), self.limb(3), 0];
        if rhs.limb(3) > 0 {
            // divrem_nby4
            divrem_nbym(&mut numerator, &mut [
                rhs.limb(0),
                rhs.limb(1),
                rhs.limb(2),
                rhs.limb(3),
            ]);
            Some((
                U256::from_limbs([numerator[4], 0, 0, 0]),
                U256::from_limbs([numerator[0], numerator[1], numerator[2], numerator[3]]),
            ))
        } else if rhs.limb(2) > 0 {
            // divrem_nby3
            divrem_nbym(&mut numerator, &mut [rhs.limb(0), rhs.limb(1), rhs.limb(2)]);
            Some((
                U256::from_limbs([numerator[3], numerator[4], 0, 0]),
                U256::from_limbs([numerator[0], numerator[1], numerator[2], 0]),
            ))
        } else if rhs.limb(1) > 0 {
            // divrem_nby2
            divrem_nbym(&mut numerator, &mut [rhs.limb(0), rhs.limb(1)]);
            Some((
                U256::from_limbs([numerator[2], numerator[3], numerator[4], 0]),
                U256::from_limbs([numerator[0], numerator[1], 0, 0]),
            ))
        } else if rhs.limb(0) > 0 {
            let remainder = divrem_nby1(&mut numerator, rhs.limb(0));
            Some((
                U256::from_limbs([numerator[0], numerator[1], numerator[2], numerator[3]]),
                U256::from_limbs([remainder, 0, 0, 0]),
            ))
        } else {
            None
        }
    }
}

/// Ring inversion.
// TODO: Make custom trait that adds `fn is_unit(&self) -> bool`.
// TODO: Implement Inv for u8..u128
impl Inv for &U256 {
    type Output = Option<U256>;

    /// Computes the inverse modulo 2^256
    fn inv(self) -> Self::Output {
        if !self.bit(0) {
            None
        } else {
            // Invert using Hensel lifted Newton-Rhapson iteration
            // See: https://arxiv.org/abs/1303.0328
            // r[2] = 3 * self XOR 2 mod 2^4
            // r[n+1] = r[n] * (1 - self * r[n]) mod 2^(2^n)
            let c = Wrapping(self.limb(0));
            let mut r: Wrapping<u64> = (Wrapping(3) * c) ^ Wrapping(2); // mod 2^4
            r *= Wrapping(2) - c * r; // mod 2^8
            r *= Wrapping(2) - c * r; // mod 2^16
            r *= Wrapping(2) - c * r; // mod 2^32
            r *= Wrapping(2) - c * r; // mod 2^64
            let mut r = Wrapping(u128::from(r.0));
            r *= Wrapping(2) - Wrapping(self.as_u128()) * r; // mod 2^128
            let mut r = U256::from(r.0);
            r *= &(U256::from(2_u64) - &(r.clone() * self)); // mod 2^256
            Some(r)
        }
    }
}

impl DivAssign<&U256> for U256 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: &Self) {
        let (q, _r) = self.div_rem(rhs).unwrap();
        *self = q;
    }
}

impl RemAssign<&U256> for U256 {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: &Self) {
        let (_q, r) = self.div_rem(rhs).unwrap();
        *self = r;
    }
}

noncommutative_binop!(U256, Div, div, DivAssign, div_assign);
noncommutative_binop!(U256, Rem, rem, RemAssign, rem_assign);

// TODO: Replace literals with u256h!
#[allow(clippy::unreadable_literal)]
// Quickcheck requires pass by value
#[allow(clippy::needless_pass_by_value)]
#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_invmod256() {
        let a = U256::from_limbs([
            0xf80aa815a36a7e47,
            0x090be90cfa96712a,
            0xf52ec0a4083d2c14,
            0x05405dfd1d1c1a97,
        ]);
        let e = U256::from_limbs([
            0xf0a9a0091b3bcb77,
            0x42d3eba6084ca0de,
            0x60d848b6513392d7,
            0xdf45026654d086d6,
        ]);
        let r = a.inv().unwrap();
        assert_eq!(r, e);
    }

    #[test]
    fn test_invmod_small() {
        let n = U256::from_limbs([271, 0, 0, 0]);
        let m = U256::from_limbs([383, 0, 0, 0]);
        let i = U256::from_limbs([106, 0, 0, 0]);
        let r = n.inv_mod(&m).unwrap();
        assert_eq!(i, r);
    }

    #[test]
    fn test_invmod() {
        let m = U256::from_limbs([
            0x0000000000000001,
            0x0000000000000000,
            0x0000000000000000,
            0x0800000000000011,
        ]);
        let n = U256::from_limbs([
            0x1717f47973471ed5,
            0xe106229070982941,
            0xd82120c54277c73e,
            0x07717a21e77894e8,
        ]);
        let i = U256::from_limbs([
            0xbda5eaad406f66d1,
            0xfac4d8e66130d944,
            0x97c88939cbce8317,
            0x001752ce51d19c97,
        ]);
        let r = n.inv_mod(&m).unwrap();
        assert_eq!(i, r);
    }

    #[quickcheck]
    fn test_divrem_u64(a: U256, b: u64) -> bool {
        match a.div_rem(b) {
            None => b == 0,
            Some((q, r)) => r < b && q * &U256::from(b) + &U256::from(r) == a,
        }
    }

    #[quickcheck]
    fn test_divrem(a: U256, b: U256) -> bool {
        match a.div_rem(&b) {
            None => b == U256::ZERO,
            Some((q, r)) => r < b && q * &b + &r == a,
        }
    }

    #[quickcheck]
    fn invmod256(a: U256) -> bool {
        match a.inv() {
            None => true,
            Some(i) => a * &i == U256::ONE,
        }
    }
}
