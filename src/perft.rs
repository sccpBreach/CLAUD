//! Perft: hitung jumlah node di pohon permainan sampai kedalaman tertentu.
//! Alat validasi utama untuk movegen - kalau angkanya cocok dengan referensi
//! standar, movegen (termasuk castling/en passant/promosi/pin/check) benar.

use crate::board::Board;
use crate::movegen::gen_legal;

pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = gen_legal(board);
    let mut total = 0u64;
    for &mv in moves.iter() {
        let undo = board.make_move(mv);
        total += perft(board, depth - 1);
        board.unmake_move(mv, undo);
    }
    total
}

/// Perft per-langkah-root, untuk debug kalau ada angka yang tidak cocok
/// (bandingkan output ini dengan engine referensi seperti Stockfish `go perft N`).
pub fn perft_divide(board: &mut Board, depth: u32) -> Vec<(String, u64)> {
    let moves = gen_legal(board);
    let mut results = Vec::new();
    for &mv in moves.iter() {
        let undo = board.make_move(mv);
        let count = if depth == 0 { 1 } else { perft(board, depth - 1) };
        board.unmake_move(mv, undo);
        results.push((mv.to_uci(), count));
    }
    results
}

/// Lima posisi standar chess programming wiki - dipakai `cargo run -- perfttest`.
pub const PERFT_SUITE: &[(&str, &str, &[u64])] = &[
    (
        "startpos",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &[1, 20, 400, 8902, 197281, 4865609],
    ),
    (
        "kiwipete",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[1, 48, 2039, 97862, 4085603],
    ),
    (
        "pos3",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[1, 14, 191, 2812, 43238, 674624],
    ),
    (
        "pos4",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[1, 6, 264, 9467, 422333],
    ),
    (
        "pos5",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[1, 44, 1486, 62379, 2103487],
    ),
];

/// Jalankan seluruh suite dan return true kalau semua cocok. Print hasil ke stdout.
pub fn run_perft_suite(max_depth: usize) -> bool {
    let mut all_ok = true;
    for &(name, fen, expected) in PERFT_SUITE {
        println!("=== {name} ===");
        let mut board = Board::from_fen(fen).expect("FEN suite pasti valid");
        for (depth, &exp) in expected.iter().enumerate().take(max_depth + 1) {
            let got = perft(&mut board, depth as u32);
            let ok = got == exp;
            if !ok {
                all_ok = false;
            }
            println!(
                "  depth {depth}: expected {exp:>10}  got {got:>10}  [{}]",
                if ok { "OK" } else { "MISMATCH" }
            );
        }
    }
    all_ok
}
