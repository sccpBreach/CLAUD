//! Representasi papan catur dan logika make/unmake move.
//!
//! Catatan arsitektur: titik integrasi NNUE (pola BoardObserver di rencana desain)
//! ditandai dengan komentar `// NNUE-HOOK` di make_move/unmake_move - tinggal
//! panggil observer di titik itu nanti, tidak perlu bongkar struktur board.

use crate::attacks::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks_to, rook_attacks};
use crate::bitboard::Bitboard;
use crate::moves::{Move, MoveFlag};
use crate::types::*;

#[derive(Debug, Clone, Copy)]
pub struct Undo {
    pub castling: u8,
    pub ep_square: Option<Square>,
    pub halfmove: u16,
    /// (warna, jenis bidak, kotak) bidak yang termakan - kotaknya beda dari `to`
    /// khusus untuk en passant.
    pub captured: Option<(usize, usize, Square)>,
}

#[derive(Debug, Clone)]
pub struct Board {
    /// pieces[warna][jenis] = bitboard posisi bidak itu.
    pub pieces: [[Bitboard; NUM_PIECE_TYPES]; 2],
    pub side: usize,
    /// bit0=WK bit1=WQ bit2=BK bit3=BQ
    pub castling: u8,
    pub ep_square: Option<Square>,
    pub halfmove: u16,
    pub fullmove: u32,
}

pub const CASTLE_WK: u8 = 0b0001;
pub const CASTLE_WQ: u8 = 0b0010;
pub const CASTLE_BK: u8 = 0b0100;
pub const CASTLE_BQ: u8 = 0b1000;

pub const STARTPOS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Debug, Clone, Copy)]
pub struct NullUndo {
    ep_square: Option<Square>,
}

impl Board {
    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(format!("FEN tidak lengkap: '{fen}'"));
        }

        let mut pieces = [[Bitboard(0); NUM_PIECE_TYPES]; 2];
        let mut rank: i32 = 7;
        let mut file: i32 = 0;
        for c in parts[0].chars() {
            match c {
                '/' => {
                    rank -= 1;
                    file = 0;
                }
                d if d.is_ascii_digit() => {
                    file += d.to_digit(10).unwrap() as i32;
                }
                c => {
                    let color = if c.is_ascii_uppercase() { WHITE } else { BLACK };
                    let pt = piece_type_from_char(c).ok_or_else(|| format!("karakter FEN tidak valid: {c}"))?;
                    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                        return Err(format!("posisi FEN keluar papan pada '{c}'"));
                    }
                    let sq = make_square(file as u8, rank as u8);
                    pieces[color][pt].set(sq);
                    file += 1;
                }
            }
        }

        let side = match parts[1] {
            "w" => WHITE,
            "b" => BLACK,
            other => return Err(format!("side-to-move tidak valid: {other}")),
        };

        let mut castling = 0u8;
        if parts[2] != "-" {
            for c in parts[2].chars() {
                castling |= match c {
                    'K' => CASTLE_WK,
                    'Q' => CASTLE_WQ,
                    'k' => CASTLE_BK,
                    'q' => CASTLE_BQ,
                    _ => return Err(format!("hak castling tidak valid: {c}")),
                };
            }
        }

        let ep_square = if parts[3] == "-" {
            None
        } else {
            Some(square_from_str(parts[3]).ok_or_else(|| format!("kotak en passant tidak valid: {}", parts[3]))?)
        };

        let halfmove: u16 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let fullmove: u32 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);

        Ok(Board { pieces, side, castling, ep_square, halfmove, fullmove })
    }

    pub fn startpos() -> Board {
        Board::from_fen(STARTPOS_FEN).expect("STARTPOS_FEN pasti valid")
    }

    pub fn occupied_by(&self, color: usize) -> Bitboard {
        let mut bb = Bitboard(0);
        for pt in 0..NUM_PIECE_TYPES {
            bb |= self.pieces[color][pt];
        }
        bb
    }

    pub fn occupied(&self) -> Bitboard {
        self.occupied_by(WHITE) | self.occupied_by(BLACK)
    }

    pub fn piece_at(&self, sq: Square) -> Option<(usize, usize)> {
        for color in [WHITE, BLACK] {
            for pt in 0..NUM_PIECE_TYPES {
                if self.pieces[color][pt].contains(sq) {
                    return Some((color, pt));
                }
            }
        }
        None
    }

    pub fn king_square(&self, color: usize) -> Square {
        self.pieces[color][KING].lsb()
    }

    /// Apakah `sq` diserang oleh bidak berwarna `by_color`.
    pub fn attacked_by(&self, sq: Square, by_color: usize) -> bool {
        let occ = self.occupied();
        if !(pawn_attacks_to(sq, by_color) & self.pieces[by_color][PAWN]).is_empty() {
            return true;
        }
        if !(knight_attacks(sq) & self.pieces[by_color][KNIGHT]).is_empty() {
            return true;
        }
        if !(king_attacks(sq) & self.pieces[by_color][KING]).is_empty() {
            return true;
        }
        let diag_attackers = self.pieces[by_color][BISHOP] | self.pieces[by_color][QUEEN];
        if !(bishop_attacks(sq, occ) & diag_attackers).is_empty() {
            return true;
        }
        let orth_attackers = self.pieces[by_color][ROOK] | self.pieces[by_color][QUEEN];
        if !(rook_attacks(sq, occ) & orth_attackers).is_empty() {
            return true;
        }
        false
    }

    pub fn in_check(&self, color: usize) -> bool {
        self.attacked_by(self.king_square(color), opposite(color))
    }

    fn clear_castling_right(&mut self, sq: Square) {
        match sq {
            4 => self.castling &= !(CASTLE_WK | CASTLE_WQ),
            60 => self.castling &= !(CASTLE_BK | CASTLE_BQ),
            0 => self.castling &= !CASTLE_WQ,
            7 => self.castling &= !CASTLE_WK,
            56 => self.castling &= !CASTLE_BQ,
            63 => self.castling &= !CASTLE_BK,
            _ => {}
        }
    }

    /// "Null move": lewati giliran tanpa menggerakkan bidak apa pun. Dipakai
    /// Null Move Pruning. TIDAK boleh dipanggil saat sedang skak (caller wajib
    /// cek `in_check` dulu - langkah null waktu skak tidak masuk akal secara
    /// aturan catur maupun secara algoritma).
    pub fn make_null_move(&mut self) -> NullUndo {
        let undo = NullUndo { ep_square: self.ep_square };
        self.ep_square = None;
        self.side = opposite(self.side);
        undo
    }

    pub fn unmake_null_move(&mut self, undo: NullUndo) {
        self.side = opposite(self.side);
        self.ep_square = undo.ep_square;
    }

    /// Dipakai sebagai syarat aman Null Move Pruning: hindari NMP di endgame
    /// raja+pion murni, karena zugzwang bikin "lewati giliran" jadi asumsi
    /// yang salah (kadang benar-benar tidak ada langkah bagus, giliran lewat
    /// dianggap terbaik padahal itu ilegal dalam catur sungguhan).
    pub fn has_non_pawn_material(&self, color: usize) -> bool {
        !self.pieces[color][KNIGHT].is_empty()
            || !self.pieces[color][BISHOP].is_empty()
            || !self.pieces[color][ROOK].is_empty()
            || !self.pieces[color][QUEEN].is_empty()
    }

    /// Jalankan sebuah langkah (diasumsikan pseudo-legal, hasil dari movegen).
    /// Mengembalikan `Undo` yang wajib dipakai lewat `unmake_move` untuk membalikkan.
    pub fn make_move(&mut self, mv: Move) -> Undo {
        let us = self.side;
        let them = opposite(us);

        let (_, moved_pt) = self
            .piece_at(mv.from)
            .unwrap_or_else(|| panic!("tidak ada bidak di kotak asal {} untuk langkah {}", mv.from, mv));

        let mut undo = Undo {
            castling: self.castling,
            ep_square: self.ep_square,
            halfmove: self.halfmove,
            captured: None,
        };

        self.ep_square = None;

        if mv.flag == MoveFlag::EnPassant {
            let cap_sq = if us == WHITE { mv.to - 8 } else { mv.to + 8 };
            undo.captured = Some((them, PAWN, cap_sq));
            self.pieces[them][PAWN].clear(cap_sq); // NNUE-HOOK: on_piece_removed(them, PAWN, cap_sq)
        } else if let Some((cap_color, cap_pt)) = self.piece_at(mv.to) {
            undo.captured = Some((cap_color, cap_pt, mv.to));
            self.pieces[cap_color][cap_pt].clear(mv.to); // NNUE-HOOK: on_piece_removed(cap_color, cap_pt, mv.to)
        }

        self.pieces[us][moved_pt].clear(mv.from); // NNUE-HOOK: on_piece_removed(us, moved_pt, mv.from)
        let placed_pt = mv.promotion.unwrap_or(moved_pt);
        self.pieces[us][placed_pt].set(mv.to); // NNUE-HOOK: on_piece_added(us, placed_pt, mv.to)

        match mv.flag {
            MoveFlag::CastleKing => {
                let (rook_from, rook_to): (Square, Square) = if us == WHITE { (7, 5) } else { (63, 61) };
                self.pieces[us][ROOK].clear(rook_from); // NNUE-HOOK: removed
                self.pieces[us][ROOK].set(rook_to); // NNUE-HOOK: added
            }
            MoveFlag::CastleQueen => {
                let (rook_from, rook_to): (Square, Square) = if us == WHITE { (0, 3) } else { (56, 59) };
                self.pieces[us][ROOK].clear(rook_from); // NNUE-HOOK: removed
                self.pieces[us][ROOK].set(rook_to); // NNUE-HOOK: added
            }
            MoveFlag::DoublePush => {
                self.ep_square = Some((mv.from + mv.to) / 2);
            }
            _ => {}
        }

        self.clear_castling_right(mv.from);
        self.clear_castling_right(mv.to);

        if moved_pt == PAWN || undo.captured.is_some() {
            self.halfmove = 0;
        } else {
            self.halfmove += 1;
        }

        self.side = them;
        if us == BLACK {
            self.fullmove += 1;
        }

        undo
    }

    /// Balikkan langkah `mv` yang barusan dijalankan `make_move`, memakai `undo`
    /// yang dikembalikan saat itu. HARUS dipanggil dengan `mv`/`undo` yang sama
    /// persis dan tanpa ada make_move lain di antaranya (disiplin stack LIFO).
    pub fn unmake_move(&mut self, mv: Move, undo: Undo) {
        let us = opposite(self.side);
        self.side = us;
        if us == BLACK {
            self.fullmove -= 1;
        }
        self.castling = undo.castling;
        self.ep_square = undo.ep_square;
        self.halfmove = undo.halfmove;

        let placed_pt = self
            .piece_at(mv.to)
            .map(|(_, pt)| pt)
            .unwrap_or_else(|| panic!("tidak ada bidak di kotak tujuan {} saat unmake {}", mv.to, mv));
        self.pieces[us][placed_pt].clear(mv.to);
        let orig_pt = if mv.promotion.is_some() { PAWN } else { placed_pt };
        self.pieces[us][orig_pt].set(mv.from);

        match mv.flag {
            MoveFlag::CastleKing => {
                let (rook_from, rook_to): (Square, Square) = if us == WHITE { (7, 5) } else { (63, 61) };
                self.pieces[us][ROOK].clear(rook_to);
                self.pieces[us][ROOK].set(rook_from);
            }
            MoveFlag::CastleQueen => {
                let (rook_from, rook_to): (Square, Square) = if us == WHITE { (0, 3) } else { (56, 59) };
                self.pieces[us][ROOK].clear(rook_to);
                self.pieces[us][ROOK].set(rook_from);
            }
            _ => {}
        }

        if let Some((cap_color, cap_pt, cap_sq)) = undo.captured {
            self.pieces[cap_color][cap_pt].set(cap_sq);
        }
    }

    /// Cetak papan sederhana buat debug (command `d` di UCI).
    pub fn print(&self) {
        for rank in (0..8).rev() {
            print!("{}  ", rank + 1);
            for file in 0..8 {
                let sq = make_square(file, rank);
                match self.piece_at(sq) {
                    Some((color, pt)) => print!("{} ", piece_char(pt, color)),
                    None => print!(". "),
                }
            }
            println!();
        }
        println!("   a b c d e f g h");
        println!(
            "side={} castling={:04b} ep={:?} halfmove={} fullmove={}",
            if self.side == WHITE { "w" } else { "b" },
            self.castling,
            self.ep_square.map(square_to_str),
            self.halfmove,
            self.fullmove
        );
    }
}
