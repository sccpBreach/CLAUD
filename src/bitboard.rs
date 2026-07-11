//! Bitboard: representasi 64-bit untuk satu set kotak di papan.
//! Pemetaan LERF (Little-Endian Rank-File): bit 0 = a1, bit 7 = h1, bit 56 = a8, bit 63 = h8.

use crate::types::Square;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, Shr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bitboard(pub u64);

pub const EMPTY: Bitboard = Bitboard(0);

pub const FILE_A: Bitboard = Bitboard(0x0101_0101_0101_0101);
pub const FILE_H: Bitboard = Bitboard(0x8080_8080_8080_8080);
pub const RANK_1: Bitboard = Bitboard(0x0000_0000_0000_00FF);
pub const RANK_2: Bitboard = Bitboard(0x0000_0000_0000_FF00);
pub const RANK_4: Bitboard = Bitboard(0x0000_0000_FF00_0000);
pub const RANK_5: Bitboard = Bitboard(0x0000_00FF_0000_0000);
pub const RANK_7: Bitboard = Bitboard(0x00FF_0000_0000_0000);
pub const RANK_8: Bitboard = Bitboard(0xFF00_0000_0000_0000);

impl Bitboard {
    #[inline(always)]
    pub const fn from_square(sq: Square) -> Bitboard {
        Bitboard(1u64 << sq)
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn contains(self, sq: Square) -> bool {
        (self.0 >> sq) & 1 != 0
    }

    #[inline(always)]
    pub fn set(&mut self, sq: Square) {
        self.0 |= 1u64 << sq;
    }

    #[inline(always)]
    pub fn clear(&mut self, sq: Square) {
        self.0 &= !(1u64 << sq);
    }

    #[inline(always)]
    pub const fn popcount(self) -> u32 {
        self.0.count_ones()
    }

    /// Indeks bit-1 paling rendah (least significant bit). Panic kalau bitboard kosong.
    #[inline(always)]
    pub fn lsb(self) -> Square {
        debug_assert!(!self.is_empty(), "lsb() dipanggil pada bitboard kosong");
        self.0.trailing_zeros() as Square
    }

    /// Ambil dan hapus lsb sekaligus, dipakai dalam pola `while let Some(sq) = bb.pop_lsb()`.
    #[inline(always)]
    pub fn pop_lsb(&mut self) -> Option<Square> {
        if self.0 == 0 {
            return None;
        }
        let sq = self.0.trailing_zeros() as Square;
        self.0 &= self.0 - 1;
        Some(sq)
    }

    #[inline(always)]
    pub fn shift_north(self) -> Bitboard {
        Bitboard(self.0 << 8)
    }

    #[inline(always)]
    pub fn shift_south(self) -> Bitboard {
        Bitboard(self.0 >> 8)
    }

    #[inline(always)]
    pub fn shift_east(self) -> Bitboard {
        Bitboard((self.0 & !FILE_H.0) << 1)
    }

    #[inline(always)]
    pub fn shift_west(self) -> Bitboard {
        Bitboard((self.0 & !FILE_A.0) >> 1)
    }

    #[inline(always)]
    pub fn shift_north_east(self) -> Bitboard {
        Bitboard((self.0 & !FILE_H.0) << 9)
    }

    #[inline(always)]
    pub fn shift_north_west(self) -> Bitboard {
        Bitboard((self.0 & !FILE_A.0) << 7)
    }

    #[inline(always)]
    pub fn shift_south_east(self) -> Bitboard {
        Bitboard((self.0 & !FILE_H.0) >> 7)
    }

    #[inline(always)]
    pub fn shift_south_west(self) -> Bitboard {
        Bitboard((self.0 & !FILE_A.0) >> 9)
    }
}

/// Iterator supaya bisa `for sq in bitboard { ... }`.
impl Iterator for Bitboard {
    type Item = Square;
    #[inline(always)]
    fn next(&mut self) -> Option<Square> {
        self.pop_lsb()
    }
}

impl BitOr for Bitboard {
    type Output = Bitboard;
    #[inline(always)]
    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 | rhs.0)
    }
}
impl BitOrAssign for Bitboard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Bitboard) {
        self.0 |= rhs.0;
    }
}
impl BitAnd for Bitboard {
    type Output = Bitboard;
    #[inline(always)]
    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 & rhs.0)
    }
}
impl BitAndAssign for Bitboard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Bitboard) {
        self.0 &= rhs.0;
    }
}
impl BitXor for Bitboard {
    type Output = Bitboard;
    #[inline(always)]
    fn bitxor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for Bitboard {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Bitboard) {
        self.0 ^= rhs.0;
    }
}
impl Not for Bitboard {
    type Output = Bitboard;
    #[inline(always)]
    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}
impl Shl<u32> for Bitboard {
    type Output = Bitboard;
    #[inline(always)]
    fn shl(self, rhs: u32) -> Bitboard {
        Bitboard(self.0 << rhs)
    }
}
impl Shr<u32> for Bitboard {
    type Output = Bitboard;
    #[inline(always)]
    fn shr(self, rhs: u32) -> Bitboard {
        Bitboard(self.0 >> rhs)
    }
}

impl std::fmt::Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let sq = (rank * 8 + file) as Square;
                write!(f, "{}", if self.contains(sq) { '1' } else { '.' })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
