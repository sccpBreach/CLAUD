//! Protokol UCI + command `bench` (wajib untuk OpenBench).

use std::io::{self, BufRead, Write};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::movegen::gen_legal;
use crate::moves::Move;
use crate::perft::perft_divide;
use crate::search::Searcher;
use crate::types::{piece_type_from_char, square_from_str};

const DEFAULT_TT_MB: usize = 16;
const DEFAULT_MAX_DEPTH: i32 = 64;

pub fn run_uci_loop() {
    let mut board = Board::startpos();
    let mut searcher = Searcher::new(DEFAULT_TT_MB);
    let stdin = io::stdin();

    println!("id name engine (Fase 1 - alpha-beta dasar)");
    println!("id author kamu");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut tokens = line.split_whitespace();
        let cmd = match tokens.next() {
            Some(c) => c,
            None => continue,
        };

        match cmd {
            "uci" => {
                println!("id name engine (Fase 1 - alpha-beta dasar)");
                println!("id author kamu");
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Threads type spin default 1 min 1 max 1");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                board = Board::startpos();
                searcher.clear_tt();
                searcher.clear_history();
            }
            "position" => handle_position(&mut board, tokens.collect::<Vec<_>>()),
            "setoption" => handle_setoption(&mut searcher, tokens.collect::<Vec<_>>()),
            "go" => handle_go(&mut board, &mut searcher, tokens.collect::<Vec<_>>()),
            "d" => board.print(),
            "perft" => {
                if let Some(depth_str) = tokens.next() {
                    if let Ok(depth) = depth_str.parse::<u32>() {
                        run_perft_command(&mut board, depth);
                    }
                }
            }
            "bench" => run_bench(),
            "quit" => break,
            _ => { /* command tidak dikenal - diabaikan, sesuai kebiasaan UCI */ }
        }
        let _ = io::stdout().flush();
    }
}

fn handle_position(board: &mut Board, tokens: Vec<&str>) {
    let mut idx = 0;
    if idx >= tokens.len() {
        return;
    }

    if tokens[idx] == "startpos" {
        *board = Board::startpos();
        idx += 1;
    } else if tokens[idx] == "fen" {
        idx += 1;
        let fen_start = idx;
        while idx < tokens.len() && tokens[idx] != "moves" {
            idx += 1;
        }
        let fen = tokens[fen_start..idx].join(" ");
        match Board::from_fen(&fen) {
            Ok(b) => *board = b,
            Err(e) => {
                eprintln!("info string FEN tidak valid: {e}");
                return;
            }
        }
    } else {
        return;
    }

    if idx < tokens.len() && tokens[idx] == "moves" {
        idx += 1;
        while idx < tokens.len() {
            apply_uci_move(board, tokens[idx]);
            idx += 1;
        }
    }
}

/// Cari langkah legal yang cocok dengan notasi UCI ("e2e4", "e7e8q", dst) lalu jalankan.
fn apply_uci_move(board: &mut Board, uci: &str) {
    if uci.len() < 4 {
        return;
    }
    let from = match square_from_str(&uci[0..2]) {
        Some(s) => s,
        None => return,
    };
    let to = match square_from_str(&uci[2..4]) {
        Some(s) => s,
        None => return,
    };
    let promo = if uci.len() >= 5 {
        piece_type_from_char(uci.as_bytes()[4] as char)
    } else {
        None
    };

    let legal = gen_legal(board);
    let found: Option<Move> = legal
        .iter()
        .find(|m| m.from == from && m.to == to && m.promotion == promo)
        .copied();

    if let Some(mv) = found {
        board.make_move(mv);
    } else {
        eprintln!("info string langkah '{uci}' tidak legal di posisi ini, diabaikan");
    }
}

/// `setoption name <NAME> value <VALUE>`. Cuma `Hash` yang benar-benar
/// berpengaruh di Fase 1 (single-thread) - `Threads` diterima tapi diabaikan
/// (bukan error; wajar dalam UCI kalau engine belum dukung suatu opsi).
fn handle_setoption(searcher: &mut Searcher, tokens: Vec<&str>) {
    let mut name_parts: Vec<&str> = Vec::new();
    let mut value_parts: Vec<&str> = Vec::new();
    let mut section = 0u8; // 0=belum ketemu keyword, 1=nama, 2=nilai

    for tok in tokens {
        match tok {
            "name" => section = 1,
            "value" => section = 2,
            _ => match section {
                1 => name_parts.push(tok),
                2 => value_parts.push(tok),
                _ => {}
            },
        }
    }

    let name = name_parts.join(" ");
    let value = value_parts.join(" ");

    match name.as_str() {
        "Hash" => {
            if let Ok(mb) = value.parse::<usize>() {
                searcher.resize_tt(mb.clamp(1, 1024));
            }
        }
        "Threads" => { /* Fase 1 single-thread - diterima, sengaja diabaikan */ }
        _ => {}
    }
}

/// Manajemen waktu SANGAT sederhana untuk Fase 1: alokasikan kira-kira 1/30
/// sisa waktu plus separuh increment, dengan buffer aman 100ms. Rencana desain
/// kita (soft/hard bound + voting antar-thread) adalah penghalusan lanjutan
/// setelah threading (Lazy SMP) ada - belum relevan di single-thread Fase 1.
fn compute_deadline(tokens: &[&str], side_is_white: bool) -> (Option<Instant>, i32) {
    let mut max_depth = DEFAULT_MAX_DEPTH;
    let mut movetime_ms: Option<u64> = None;
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: u64 = 0;
    let mut binc: u64 = 0;
    let mut infinite = false;

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                max_depth = tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_MAX_DEPTH);
                i += 2;
            }
            "movetime" => {
                movetime_ms = tokens.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "wtime" => {
                wtime = tokens.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "btime" => {
                btime = tokens.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "winc" => {
                winc = tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "binc" => {
                binc = tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "infinite" => {
                infinite = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if infinite {
        return (None, max_depth);
    }

    if let Some(mt) = movetime_ms {
        return (Some(Instant::now() + Duration::from_millis(mt)), max_depth);
    }

    let (my_time, my_inc) = if side_is_white { (wtime, winc) } else { (btime, binc) };

    let deadline = my_time.map(|t| {
        let raw_budget = (t / 30).saturating_add(my_inc / 2);
        let safety_cap = t.saturating_sub(100);
        let budget = raw_budget.min(safety_cap).max(50);
        Instant::now() + Duration::from_millis(budget)
    });

    (deadline, max_depth)
}

fn handle_go(board: &mut Board, searcher: &mut Searcher, tokens: Vec<&str>) {
    let (deadline, max_depth) = compute_deadline(&tokens, board.side == crate::types::WHITE);
    let result = searcher.search(board, max_depth, deadline);
    match result.best_move {
        Some(mv) => println!("bestmove {mv}"),
        None => println!("bestmove 0000"),
    }
}

fn run_perft_command(board: &mut Board, depth: u32) {
    let t0 = Instant::now();
    let divide = perft_divide(board, depth);
    let mut total = 0u64;
    for (uci, count) in &divide {
        println!("{uci}: {count}");
        total += count;
    }
    let dt = t0.elapsed();
    println!();
    println!("Nodes searched: {total}");
    let nps = if dt.as_secs_f64() > 0.0 { (total as f64 / dt.as_secs_f64()) as u64 } else { 0 };
    println!("({:.3}s, {} nps)", dt.as_secs_f64(), nps);
}

/// Command `bench`. Format output SENGAJA mengikuti syarat OpenBench:
/// "<N> nodes <M> nps" (baris terakhir yang di-print).
pub fn run_bench() {
    let positions: &[&str] = &[
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    ];
    let bench_depth = 6;

    let t0 = Instant::now();
    let mut total_nodes: u64 = 0;
    for fen in positions {
        let mut board = Board::from_fen(fen).expect("FEN bench pasti valid");
        let mut searcher = Searcher::new(DEFAULT_TT_MB);
        let result = searcher.search(&mut board, bench_depth, None);
        total_nodes += result.nodes;
    }
    let dt = t0.elapsed();
    let nps = if dt.as_secs_f64() > 0.0 { (total_nodes as f64 / dt.as_secs_f64()) as u64 } else { 0 };

    println!("{total_nodes} nodes {nps} nps");
}
