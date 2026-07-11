//! Move generation. Struktur dan urutan logika ini SENGAJA meniru persis
//! `validate/refgen.py` yang sudah divalidasi lolos perft di 5 posisi standar
//! (startpos, kiwipete, pos3, pos4, pos5) sampai jutaan node - supaya risiko
//! salah transkripsi seminimal mungkin.
//!
//! Pendekatan: pseudo-legal generation, lalu filter legal dengan cara
//! make_move -> cek raja sendiri diserang? -> unmake_move. Ini BUKAN cara
//! tercepat (lihat rencana arsitektur: versi final pakai pin/checker bitboard
//! seperti Reckless), tapi paling gampang dipastikan benar dulu.

use crate::attacks::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks};
use crate::bitboard::{Bitboard, RANK_1, RANK_2, RANK_7, RANK_8};
use crate::board::{Board, CASTLE_BK, CASTLE_BQ, CASTLE_WK, CASTLE_WQ};
use crate::moves::{Move, MoveFlag, MoveList};
use crate::types::*;

fn gen_pawn_moves(board: &Board, us: usize, list: &mut MoveList) {
    let them = opposite(us);
    let occ = board.occupied();
    let enemy = board.occupied_by(them);
    let (start_rank, promo_rank, push_dir): (Bitboard, Bitboard, i32) =
        if us == WHITE { (RANK_2, RANK_8, 8) } else { (RANK_7, RANK_1, -8) };

    let mut pawns = board.pieces[us][PAWN];
    while let Some(frm) = pawns.pop_lsb() {
        let to1_i = frm as i32 + push_dir;
        if (0..64).contains(&to1_i) {
            let to1 = to1_i as Square;
            if !occ.contains(to1) {
                if promo_rank.contains(to1) {
                    for &pp in &[QUEEN, ROOK, BISHOP, KNIGHT] {
                        list.push(Move::new_promotion(frm, to1, pp, false));
                    }
                } else {
                    list.push(Move::new(frm, to1, MoveFlag::Quiet));
                    if start_rank.contains(frm) {
                        let to2_i = frm as i32 + 2 * push_dir;
                        if (0..64).contains(&to2_i) {
                            let to2 = to2_i as Square;
                            if !occ.contains(to2) {
                                list.push(Move::new(frm, to2, MoveFlag::DoublePush));
                            }
                        }
                    }
                }
            }
        }

        let mut atk = pawn_attacks(frm, us);
        while let Some(to) = atk.pop_lsb() {
            if enemy.contains(to) {
                if promo_rank.contains(to) {
                    for &pp in &[QUEEN, ROOK, BISHOP, KNIGHT] {
                        list.push(Move::new_promotion(frm, to, pp, true));
                    }
                } else {
                    list.push(Move::new(frm, to, MoveFlag::Capture));
                }
            } else if board.ep_square == Some(to) {
                list.push(Move::new(frm, to, MoveFlag::EnPassant));
            }
        }
    }
}

fn gen_knight_moves(board: &Board, us: usize, list: &mut MoveList) {
    let own = board.occupied_by(us);
    let occ = board.occupied();
    let mut knights = board.pieces[us][KNIGHT];
    while let Some(frm) = knights.pop_lsb() {
        let mut targets = knight_attacks(frm) & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag = if occ.contains(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            list.push(Move::new(frm, to, flag));
        }
    }
}

fn gen_slider_moves(board: &Board, us: usize, piece_type: usize, list: &mut MoveList) {
    let own = board.occupied_by(us);
    let occ = board.occupied();
    let mut pieces = board.pieces[us][piece_type];
    while let Some(frm) = pieces.pop_lsb() {
        let attacks = match piece_type {
            BISHOP => bishop_attacks(frm, occ),
            ROOK => rook_attacks(frm, occ),
            QUEEN => queen_attacks(frm, occ),
            _ => unreachable!("gen_slider_moves dipanggil dengan piece_type bukan slider"),
        };
        let mut targets = attacks & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag = if occ.contains(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            list.push(Move::new(frm, to, flag));
        }
    }
}

fn gen_king_moves(board: &Board, us: usize, list: &mut MoveList) {
    let own = board.occupied_by(us);
    let occ = board.occupied();
    let ksq = board.king_square(us);
    let mut targets = king_attacks(ksq) & !own;
    while let Some(to) = targets.pop_lsb() {
        let flag = if occ.contains(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
        list.push(Move::new(ksq, to, flag));
    }
}

fn gen_castling_moves(board: &Board, us: usize, list: &mut MoveList) {
    let occ = board.occupied();
    let them = opposite(us);
    if us == WHITE {
        if board.castling & CASTLE_WK != 0
            && (occ & (Bitboard::from_square(5) | Bitboard::from_square(6))).is_empty()
            && !board.in_check(WHITE)
            && !board.attacked_by(5, them)
            && !board.attacked_by(6, them)
        {
            list.push(Move::new(4, 6, MoveFlag::CastleKing));
        }
        if board.castling & CASTLE_WQ != 0
            && (occ & (Bitboard::from_square(1) | Bitboard::from_square(2) | Bitboard::from_square(3))).is_empty()
            && !board.in_check(WHITE)
            && !board.attacked_by(3, them)
            && !board.attacked_by(2, them)
        {
            list.push(Move::new(4, 2, MoveFlag::CastleQueen));
        }
    } else {
        if board.castling & CASTLE_BK != 0
            && (occ & (Bitboard::from_square(61) | Bitboard::from_square(62))).is_empty()
            && !board.in_check(BLACK)
            && !board.attacked_by(61, them)
            && !board.attacked_by(62, them)
        {
            list.push(Move::new(60, 62, MoveFlag::CastleKing));
        }
        if board.castling & CASTLE_BQ != 0
            && (occ & (Bitboard::from_square(57) | Bitboard::from_square(58) | Bitboard::from_square(59))).is_empty()
            && !board.in_check(BLACK)
            && !board.attacked_by(59, them)
            && !board.attacked_by(58, them)
        {
            list.push(Move::new(60, 58, MoveFlag::CastleQueen));
        }
    }
}

pub fn gen_pseudo_legal(board: &Board, list: &mut MoveList) {
    let us = board.side;
    gen_pawn_moves(board, us, list);
    gen_knight_moves(board, us, list);
    gen_slider_moves(board, us, BISHOP, list);
    gen_slider_moves(board, us, ROOK, list);
    gen_slider_moves(board, us, QUEEN, list);
    gen_king_moves(board, us, list);
    gen_castling_moves(board, us, list);
}

/// Generate hanya langkah yang benar-benar LEGAL (raja sendiri tidak dalam skak
/// setelah langkah dijalankan). Butuh `&mut Board` karena internal pakai
/// make_move/unmake_move sebagai filter.
pub fn gen_legal(board: &mut Board) -> MoveList {
    let mut pseudo = MoveList::new();
    gen_pseudo_legal(&*board, &mut pseudo);

    let mut legal = MoveList::new();
    let us = board.side;
    for &mv in pseudo.iter() {
        let undo = board.make_move(mv);
        if !board.in_check(us) {
            legal.push(mv);
        }
        board.unmake_move(mv, undo);
    }
    legal
}
