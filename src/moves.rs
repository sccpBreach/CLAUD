//! Representasi langkah catur.
//!
//! Catatan desain: sengaja pakai struct field eksplisit (bukan encoding 16-bit
//! yang dipadatkan) untuk Fase 0 - lebih gampang dipastikan benar tanpa compiler
//! di tangan. Memadatkan ke u16 (pola umum di engine top: 6 bit from + 6 bit to +
//! 4 bit flag) adalah optimisasi memori yang aman dilakukan belakangan, setelah
//! perft tervalidasi dengan representasi yang jelas ini.

use crate::types::{square_to_str, Square, BISHOP, KNIGHT, QUEEN, ROOK};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag {
    Quiet,
    Capture,
    DoublePush,
    CastleKing,
    CastleQueen,
    EnPassant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    /// Some(PAWN..=QUEEN) kalau promosi (nilainya salah satu dari KNIGHT/BISHOP/ROOK/QUEEN).
    pub promotion: Option<usize>,
    pub flag: MoveFlag,
}

impl Move {
    pub fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        Move { from, to, promotion: None, flag }
    }

    pub fn new_promotion(from: Square, to: Square, promotion: usize, is_capture: bool) -> Self {
        Move {
            from,
            to,
            promotion: Some(promotion),
            flag: if is_capture { MoveFlag::Capture } else { MoveFlag::Quiet },
        }
    }

    pub fn is_capture(&self) -> bool {
        matches!(self.flag, MoveFlag::Capture | MoveFlag::EnPassant)
    }

    /// Format notasi UCI ("e2e4", "e7e8q", dst).
    pub fn to_uci(&self) -> String {
        let mut s = format!("{}{}", square_to_str(self.from), square_to_str(self.to));
        if let Some(p) = self.promotion {
            let c = match p {
                KNIGHT => 'n',
                BISHOP => 'b',
                ROOK => 'r',
                QUEEN => 'q',
                _ => '?',
            };
            s.push(c);
        }
        s
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

/// Daftar langkah dengan kapasitas tetap di stack (hindari alokasi heap per node).
/// 256 lebih dari cukup - jumlah langkah legal maksimum yang pernah tercatat di
/// posisi catur legal adalah 218.
pub const MAX_MOVES: usize = 256;

pub struct MoveList {
    moves: [Option<Move>; MAX_MOVES],
    len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        MoveList { moves: [None; MAX_MOVES], len: 0 }
    }

    pub fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES, "MoveList penuh - ada bug movegen");
        self.moves[self.len] = Some(m);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.moves[..self.len].iter().filter_map(|m| m.as_ref())
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}
