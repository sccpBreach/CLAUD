//! Generate bitboard serangan tiap jenis bidak.
//!
//! Catatan desain (Fase 0 - prioritas benar dulu, cepat belakangan):
//! - Knight/king/pawn dihitung langsung tiap panggilan (bukan lookup table precomputed).
//!   Ini sengaja: menghindari kompleksitas inisialisasi statis di awal. Optimisasi ke
//!   tabel precomputed (const/build.rs, sesuai rencana arsitektur) adalah langkah lanjutan
//!   yang aman dilakukan SETELAH perft tervalidasi dengan implementasi sederhana ini.
//! - Bishop/rook masih ray-casting biasa, BUKAN magic bitboards. Magic bitboards adalah
//!   optimisasi Fase 0.5, bukan syarat kebenaran.

use crate::bitboard::Bitboard;
use crate::types::{file_of, make_square, rank_of, Square, BLACK, WHITE};

const KNIGHT_DELTAS: [(i32, i32); 8] = [
    (1, 2), (2, 1), (2, -1), (1, -2),
    (-1, -2), (-2, -1), (-2, 1), (-1, 2),
];

const KING_DELTAS: [(i32, i32); 8] = [
    (1, 0), (1, 1), (0, 1), (-1, 1),
    (-1, 0), (-1, -1), (0, -1), (1, -1),
];

const BISHOP_DIRS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ROOK_DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

#[inline]
fn delta_attacks(sq: Square, deltas: &[(i32, i32)]) -> Bitboard {
    let f0 = file_of(sq) as i32;
    let r0 = rank_of(sq) as i32;
    let mut bb = Bitboard(0);
    for (df, dr) in deltas {
        let f = f0 + df;
        let r = r0 + dr;
        if (0..8).contains(&f) && (0..8).contains(&r) {
            bb.set(make_square(f as u8, r as u8));
        }
    }
    bb
}

pub fn knight_attacks(sq: Square) -> Bitboard {
    delta_attacks(sq, &KNIGHT_DELTAS)
}

pub fn king_attacks(sq: Square) -> Bitboard {
    delta_attacks(sq, &KING_DELTAS)
}

/// Kotak yang DISERANG oleh bidak pion berwarna `color` yang berdiri di `sq`
/// (bukan kotak yang bisa dilangkahi maju).
pub fn pawn_attacks(sq: Square, color: usize) -> Bitboard {
    let f0 = file_of(sq) as i32;
    let r0 = rank_of(sq) as i32;
    let dr: i32 = if color == WHITE { 1 } else { -1 };
    let mut bb = Bitboard(0);
    for df in [-1i32, 1i32] {
        let f = f0 + df;
        let r = r0 + dr;
        if (0..8).contains(&f) && (0..8).contains(&r) {
            bb.set(make_square(f as u8, r as u8));
        }
    }
    bb
}

fn sliding_attacks(sq: Square, occupied: Bitboard, dirs: &[(i32, i32)]) -> Bitboard {
    let f0 = file_of(sq) as i32;
    let r0 = rank_of(sq) as i32;
    let mut bb = Bitboard(0);
    for (df, dr) in dirs {
        let mut f = f0 + df;
        let mut r = r0 + dr;
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let target = make_square(f as u8, r as u8);
            bb.set(target);
            if occupied.contains(target) {
                break; // berhenti di blocker pertama (termasuk kotaknya, karena capture legal)
            }
            f += df;
            r += dr;
        }
    }
    bb
}

pub fn bishop_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    sliding_attacks(sq, occupied, &BISHOP_DIRS)
}

pub fn rook_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    sliding_attacks(sq, occupied, &ROOK_DIRS)
}

pub fn queen_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    bishop_attacks(sq, occupied) | rook_attacks(sq, occupied)
}

/// Dipakai untuk cek "apakah `sq` diserang oleh bidak pion berwarna `by_color`":
/// pakai simetri - hitung serangan pion lawan warna dari `sq`, lalu cocokkan
/// dengan bitboard pion `by_color` yang sebenarnya.
#[inline]
pub fn pawn_attacks_to(sq: Square, by_color: usize) -> Bitboard {
    pawn_attacks(sq, 1 - by_color)
}

#[allow(dead_code)]
pub fn opposite_color(c: usize) -> usize {
    if c == WHITE { BLACK } else { WHITE }
}
