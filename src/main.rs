mod attacks;
mod bitboard;
mod board;
mod eval;
mod movegen;
mod moves;
mod perft;
mod search;
mod tt;
mod types;
mod uci;
mod zobrist;

use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        uci::run_uci_loop();
        return;
    }

    match args[0].as_str() {
        "bench" => uci::run_bench(),
        "perfttest" => {
            let ok = perft::run_perft_suite(5);
            println!();
            println!("{}", if ok { "SEMUA COCOK" } else { "ADA YANG TIDAK COCOK" });
            std::process::exit(if ok { 0 } else { 1 });
        }
        "perft" => {
            let depth: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
            let mut board = board::Board::startpos();
            let t0 = std::time::Instant::now();
            let nodes = perft::perft(&mut board, depth);
            let dt = t0.elapsed();
            let nps = if dt.as_secs_f64() > 0.0 { (nodes as f64 / dt.as_secs_f64()) as u64 } else { 0 };
            let secs = dt.as_secs_f64();
            println!("perft({depth}) = {nodes}  ({secs:.3}s, {nps} nps)");
        }
        _ => {
            eprintln!("Argumen tidak dikenal: {}. Jalankan tanpa argumen untuk mode UCI.", args[0]);
        }
    }
}
