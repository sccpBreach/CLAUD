//! Evaluasi statis posisi: material + piece-square table kecil.
//!
//! Ini SENGAJA sederhana (cuma pion & kuda punya PST, sisanya material saja) -
//! sesuai rencana arsitektur: fungsi ini nanti DIGANTI oleh NNUE di Fase 2,
//! jadi tidak ada gunanya menghabiskan waktu menyempurnakan HCE di sini.
//! Tujuannya cuma supaya search di Fase 1 punya sinyal evaluasi yang masuk akal
//! untuk dites, bukan untuk kekuatan main jangka panjang.

use crate::board::Board;
use crate::types::*;

pub const PIECE_VALUE: [i32; NUM_PIECE_TYPES] = [100, 320, 330, 500, 900, 0];

#[rustfmt::skip]
const PAWN_PST: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10,-20,-20, 10, 10,  5,
     5, -5,-10,  0,  0,-10, -5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5,  5, 10, 25, 25, 10,  5,  5,
    10, 10, 20, 30, 30, 20, 10, 10,
    50, 50, 50, 50, 50, 50, 50, 50,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[inline]
fn pst_value(piece_type: usize, sq: Square, color: usize) -> i32 {
    let idx = if color == WHITE { sq } else { sq ^ 56 } as usize; // cermin vertikal untuk hitam
    match piece_type {
        PAWN => PAWN_PST[idx],
        KNIGHT => KNIGHT_PST[idx],
        _ => 0,
    }
}

/// Skor dari sudut pandang side-to-move (konvensi negamax: makin besar makin
/// bagus untuk pihak yang sedang jalan).
pub fn evaluate(board: &Board) -> i32 {
    let mut score: i32 = 0;
    for color in [WHITE, BLACK] {
        let sign: i32 = if color == WHITE { 1 } else { -1 };
        for pt in 0..NUM_PIECE_TYPES {
            let mut bb = board.pieces[color][pt];
            while let Some(sq) = bb.pop_lsb() {
                score += sign * PIECE_VALUE[pt];
                score += sign * pst_value(pt, sq, color);
            }
        }
    }
    if board.side == WHITE {
        score
    } else {
        -score
    }
}
