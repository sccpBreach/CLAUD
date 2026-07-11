//! Tipe-tipe dasar yang dipakai di seluruh engine.

pub const WHITE: usize = 0;
pub const BLACK: usize = 1;

pub const PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const ROOK: usize = 3;
pub const QUEEN: usize = 4;
pub const KING: usize = 5;

pub const NUM_PIECE_TYPES: usize = 6;

/// Square 0..=63, pemetaan Little-Endian Rank-File (LERF):
/// a1=0, b1=1, ..., h1=7, a2=8, ..., h8=63.
pub type Square = u8;

#[inline(always)]
pub const fn make_square(file: u8, rank: u8) -> Square {
    rank * 8 + file
}

#[inline(always)]
pub const fn file_of(sq: Square) -> u8 {
    sq % 8
}

#[inline(always)]
pub const fn rank_of(sq: Square) -> u8 {
    sq / 8
}

#[inline(always)]
pub const fn opposite(color: usize) -> usize {
    color ^ 1
}

/// Ubah notasi aljabar ("e4") jadi Square. Panic kalau formatnya tidak valid
/// (dipakai untuk parsing FEN/UCI yang sudah divalidasi lebih dulu).
pub fn square_from_str(s: &str) -> Option<Square> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 {
        return None;
    }
    let file = bytes[0].checked_sub(b'a')?;
    let rank = bytes[1].checked_sub(b'1')?;
    if file > 7 || rank > 7 {
        return None;
    }
    Some(make_square(file, rank))
}

pub fn square_to_str(sq: Square) -> String {
    let file = (b'a' + file_of(sq)) as char;
    let rank = (b'1' + rank_of(sq)) as char;
    format!("{file}{rank}")
}

pub fn piece_char(piece_type: usize, color: usize) -> char {
    let c = match piece_type {
        PAWN => 'p',
        KNIGHT => 'n',
        BISHOP => 'b',
        ROOK => 'r',
        QUEEN => 'q',
        KING => 'k',
        _ => unreachable!("piece_type tidak valid: {piece_type}"),
    };
    if color == WHITE {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

pub fn piece_type_from_char(c: char) -> Option<usize> {
    match c.to_ascii_lowercase() {
        'p' => Some(PAWN),
        'n' => Some(KNIGHT),
        'b' => Some(BISHOP),
        'r' => Some(ROOK),
        'q' => Some(QUEEN),
        'k' => Some(KING),
        _ => None,
    }
}
