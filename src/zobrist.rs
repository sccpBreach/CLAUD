//! Zobrist hashing: key 64-bit untuk tiap posisi, dipakai transposition table.
//!
//! Catatan desain (Fase 1): key dihitung FRESH dari papan tiap kali dibutuhkan
//! (bukan dipelihara incremental lewat make/unmake). Ini lebih lambat tapi jauh
//! lebih gampang dipastikan benar tanpa compiler di tangan. Upgrade ke
//! incremental update (XOR masuk/keluar tiap make_move/unmake_move, mirip pola
//! akumulator NNUE) adalah optimisasi lanjutan yang aman dilakukan setelah TT
//! terbukti bekerja benar.

use crate::board::Board;
use crate::types::{NUM_PIECE_TYPES, WHITE};
use std::sync::OnceLock;

pub struct ZobristKeys {
    piece_square: [[[u64; 64]; NUM_PIECE_TYPES]; 2],
    side_to_move: u64,
    castling: [u64; 16],
    ep_file: [u64; 8],
}

/// xorshift64 sederhana - deterministik, cuma butuh angka yang "cukup acak"
/// supaya jarang tabrakan, bukan butuh keamanan kriptografis.
fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

impl ZobristKeys {
    fn new() -> ZobristKeys {
        let mut seed: u64 = 0x9E3779B97F4A7C15; // seed tetap -> hasil deterministik antar-run
        let mut piece_square = [[[0u64; 64]; NUM_PIECE_TYPES]; 2];
        for color_table in piece_square.iter_mut() {
            for pt_table in color_table.iter_mut() {
                for sq_val in pt_table.iter_mut() {
                    *sq_val = next_rand(&mut seed);
                }
            }
        }
        let side_to_move = next_rand(&mut seed);
        let mut castling = [0u64; 16];
        for c in castling.iter_mut() {
            *c = next_rand(&mut seed);
        }
        let mut ep_file = [0u64; 8];
        for f in ep_file.iter_mut() {
            *f = next_rand(&mut seed);
        }
        ZobristKeys { piece_square, side_to_move, castling, ep_file }
    }
}

static ZOBRIST: OnceLock<ZobristKeys> = OnceLock::new();

fn keys() -> &'static ZobristKeys {
    ZOBRIST.get_or_init(ZobristKeys::new)
}

/// Hitung Zobrist key untuk posisi `board` saat ini. O(jumlah bidak di papan) -
/// dipanggil fresh, bukan dipelihara incremental (lihat catatan modul di atas).
pub fn compute_key(board: &Board) -> u64 {
    let z = keys();
    let mut key: u64 = 0;

    for color in 0..2 {
        for pt in 0..NUM_PIECE_TYPES {
            let mut bb = board.pieces[color][pt];
            while let Some(sq) = bb.pop_lsb() {
                key ^= z.piece_square[color][pt][sq as usize];
            }
        }
    }

    if board.side == WHITE {
        key ^= z.side_to_move;
    }

    key ^= z.castling[(board.castling & 0x0F) as usize];

    if let Some(ep) = board.ep_square {
        let file = ep % 8;
        key ^= z.ep_file[file as usize];
    }

    key
}
