//! Alpha-beta search dengan iterative deepening, quiescence search, dan empat
//! teknik pruning inti: Reverse Futility Pruning, Razoring, Null Move Pruning,
//! dan Late Move Reductions (+ Principal Variation Search re-search).
//!
//! Validasi: alpha-beta MURNI (tanpa RFP/razoring/NMP/LMR) sudah divalidasi
//! identik dengan negamax polos (lihat search.rs versi Fase 1 awal / catatan
//! desain). Empat teknik tambahan di atas itu SENGAJA "unsafe pruning" -
//! mereka mengorbankan sedikit akurasi demi kecepatan, jadi TIDAK diharapkan
//! menghasilkan skor identik dengan versi tanpa pruning. Yang divalidasi
//! (lewat referensi Python) untuk teknik-teknik ini: (1) tidak merusak solusi
//! taktis sederhana yang jelas [contoh: menangkap ratu tak terjaga tetap
//! ditemukan di berbagai kedalaman], (2) jumlah node yang dicari benar-benar
//! turun signifikan dibanding alpha-beta polos (bukti pruning-nya aktif).
//! Validasi "beneran menaikkan kekuatan main" perlu SPRT lewat OpenBench -
//! belum bisa dilakukan sampai infrastruktur itu ada (Fase 2).
//!
//! Simplifikasi yang masih ada di Fase 1 (lihat komentar TODO tersebar):
//! - Skor mate belum disesuaikan saat keluar-masuk TT (TT mate-score
//!   adjustment) - simplifikasi yang diketahui, jarang berdampak di kedalaman
//!   dangkal Fase 1.
//! - Reduksi LMR pakai formula tetap sederhana, belum di-tuning SPSA.
//! - History belum ada "gravity"/malus untuk move yang dicoba tapi gagal
//!   cutoff - cuma nambah skor saat cutoff, belum mengurangi yang tidak
//!   berhasil. Penghalusan lanjutan, bukan syarat kebenaran.

use std::time::Instant;

use crate::board::Board;
use crate::eval::{evaluate, PIECE_VALUE};
use crate::movegen::gen_legal;
use crate::moves::{Move, MoveFlag, MoveList};
use crate::tt::{Bound, TranspositionTable};
use crate::types::PAWN;
use crate::zobrist::compute_key;

pub const INFINITY: i32 = 32_000;
pub const MATE_SCORE: i32 = 30_000;
/// Batas aman ply pencarian - jauh di atas DEFAULT_MAX_DEPTH manapun yang
/// realistis. Dipakai untuk ukuran array killer move, dengan bounds-check
/// eksplisit tiap akses (lihat `killers_at`) supaya tidak ada risiko
/// index-out-of-bounds walau ply tak terduga besar (mis. dari ekstensi).
pub const MAX_PLY: usize = 128;

const NODE_CHECK_MASK: u64 = 2047; // cek waktu tiap 2048 node (pola Reckless), hindari overhead Instant::now() tiap node

pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: i32,
    pub nodes: u64,
}

pub struct Searcher {
    tt: TranspositionTable,
    nodes: u64,
    deadline: Option<Instant>,
    stopped: bool,
    /// 2 killer move per ply - quiet move yang pernah menyebabkan beta cutoff
    /// di ply itu. Direset tiap `search()` baru (khusus untuk pencarian ini).
    killers: Vec<[Option<Move>; 2]>,
    /// Skor riwayat quiet move [from][to] - dibiarkan ada antar-pencarian
    /// dalam satu game yang sama, cuma direset lewat `clear_history()`
    /// (dipanggil UCI `ucinewgame`).
    history: [[i32; 64]; 64],
}

impl Searcher {
    pub fn new(tt_size_mb: usize) -> Self {
        Searcher {
            tt: TranspositionTable::new(tt_size_mb),
            nodes: 0,
            deadline: None,
            stopped: false,
            killers: vec![[None, None]; MAX_PLY],
            history: [[0; 64]; 64],
        }
    }

    pub fn clear_tt(&mut self) {
        self.tt.clear();
    }

    /// Ganti ukuran TT (dipanggil dari `setoption name Hash value <MB>`).
    /// Mengosongkan isi TT lama - itu wajar/diharapkan tiap resize.
    pub fn resize_tt(&mut self, size_mb: usize) {
        self.tt = TranspositionTable::new(size_mb);
    }

    pub fn clear_history(&mut self) {
        self.history = [[0; 64]; 64];
    }

    #[inline]
    fn killers_at(&self, ply: u32) -> [Option<Move>; 2] {
        let idx = ply as usize;
        if idx < MAX_PLY {
            self.killers[idx]
        } else {
            [None, None]
        }
    }

    fn record_killer(&mut self, ply: u32, mv: Move) {
        let idx = ply as usize;
        if idx >= MAX_PLY {
            return;
        }
        if self.killers[idx][0] != Some(mv) {
            self.killers[idx][1] = self.killers[idx][0];
            self.killers[idx][0] = Some(mv);
        }
    }

    fn record_history(&mut self, mv: Move, depth: i32) {
        let bonus = (depth * depth).min(4096);
        let entry = &mut self.history[mv.from as usize][mv.to as usize];
        *entry = (*entry + bonus).min(1_000_000); // clamp - jaga-jaga anti overflow jangka panjang
    }

    fn check_time(&mut self) {
        if self.nodes & NODE_CHECK_MASK != 0 {
            return;
        }
        if let Some(dl) = self.deadline {
            if Instant::now() >= dl {
                self.stopped = true;
            }
        }
    }

    /// Iterative deepening sampai `max_depth` atau `deadline` (mana yang lebih dulu).
    /// Hasil dari iterasi yang TERPOTONG waktu dibuang - selalu pakai hasil
    /// iterasi terakhir yang selesai penuh (standar praktik iterative deepening).
    pub fn search(&mut self, board: &mut Board, max_depth: i32, deadline: Option<Instant>) -> SearchResult {
        self.nodes = 0;
        self.deadline = deadline;
        self.stopped = false;
        self.killers = vec![[None, None]; MAX_PLY];

        let mut result = SearchResult { best_move: None, score: 0, depth: 0, nodes: 0 };

        for depth in 1..=max_depth {
            let (score, best_move) = self.search_root(board, depth);
            if self.stopped {
                break;
            }
            result = SearchResult { best_move, score, depth, nodes: self.nodes };
            let pv_str = best_move.map(|m| m.to_uci()).unwrap_or_else(|| "(none)".to_string());
            let nodes = self.nodes;
            println!("info depth {depth} score cp {score} nodes {nodes} pv {pv_str}");
            if score.abs() >= MATE_SCORE - 1000 {
                break; // sudah ketemu mate pasti, tidak perlu cari lebih dalam
            }
        }

        result
    }

    fn search_root(&mut self, board: &mut Board, depth: i32) -> (i32, Option<Move>) {
        let legal = gen_legal(board);
        if legal.is_empty() {
            return (0, None);
        }

        let key = compute_key(board);
        let tt_move = self.tt.probe(key).and_then(|e| e.best_move);
        let killers0 = self.killers_at(0);
        let ordered = order_moves(board, &legal, tt_move, &killers0, &self.history);

        let mut alpha = -INFINITY;
        let beta = INFINITY;
        let mut best_score = -INFINITY;
        let mut best_move = ordered.first().copied();

        for mv in ordered {
            let undo = board.make_move(mv);
            let score = -self.negamax(board, depth - 1, -beta, -alpha, 1, true);
            board.unmake_move(mv, undo);

            if self.stopped {
                break;
            }

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }
            if best_score > alpha {
                alpha = best_score;
            }
        }

        self.tt.store(key, best_move, best_score, depth as i8, Bound::Exact);
        (best_score, best_move)
    }

    fn negamax(&mut self, board: &mut Board, depth: i32, mut alpha: i32, beta: i32, ply: u32, null_ok: bool) -> i32 {
        self.nodes += 1;
        self.check_time();
        if self.stopped {
            return 0;
        }

        if depth <= 0 {
            return self.quiescence(board, alpha, beta);
        }

        let in_check = board.in_check(board.side);
        let near_mate_bound = beta.abs() >= MATE_SCORE - 1000 || alpha.abs() >= MATE_SCORE - 1000;

        let key = compute_key(board);
        let mut tt_move = None;
        if let Some(entry) = self.tt.probe(key) {
            tt_move = entry.best_move;
            if entry.depth as i32 >= depth {
                match entry.bound {
                    Bound::Exact => return entry.score,
                    Bound::Lower if entry.score >= beta => return entry.score,
                    Bound::Upper if entry.score <= alpha => return entry.score,
                    _ => {}
                }
            }
        }

        // Reverse Futility Pruning ("static null move"): di depth rendah,
        // kalau eval statis sudah jauh di atas beta, asumsikan cabang ini akan
        // fail-high dan potong lebih awal. PRUNING TIDAK AMAN (heuristik) -
        // margin sengaja dibuat konservatif (80cp/ply) untuk Fase 1.
        if !in_check && !near_mate_bound && depth <= 6 {
            let margin = 80 * depth;
            let static_eval = evaluate(board);
            if static_eval - margin >= beta {
                return static_eval - margin;
            }
        }

        // Razoring: eval statis jauh di BAWAH alpha di depth rendah -> kemungkinan
        // besar tidak akan pulih, langsung percayakan ke quiescence search.
        if !in_check && depth <= 3 {
            let margin = 150 + 100 * (depth - 1);
            let static_eval = evaluate(board);
            if static_eval + margin < alpha {
                let qscore = self.quiescence(board, alpha, beta);
                if qscore < alpha {
                    return qscore;
                }
            }
        }

        // Null Move Pruning: beri lawan "giliran gratis", cari di depth
        // tereduksi. Kalau tetap fail-high, posisi ini kemungkinan besar
        // menang telak - potong. DIHINDARI saat skak (aturan catur), saat
        // tidak ada materi non-pion (rawan zugzwang), dan tidak dilakukan dua
        // kali berturut-turut (null_ok=false mencegah itu).
        if null_ok && !in_check && !near_mate_bound && depth >= 3 && board.has_non_pawn_material(board.side) {
            const R: i32 = 3;
            let null_undo = board.make_null_move();
            let null_score = -self.negamax(board, depth - 1 - R, -beta, -beta + 1, ply + 1, false);
            board.unmake_null_move(null_undo);
            if self.stopped {
                return 0;
            }
            if null_score >= beta {
                return beta;
            }
        }

        let legal = gen_legal(board);
        if legal.is_empty() {
            return if in_check { -MATE_SCORE + ply as i32 } else { 0 };
        }

        let killers_here = self.killers_at(ply);
        let ordered = order_moves(board, &legal, tt_move, &killers_here, &self.history);
        let orig_alpha = alpha;
        let mut best_score = -INFINITY;
        let mut best_move = None;

        for (i, mv) in ordered.into_iter().enumerate() {
            let is_capture = mv.is_capture();
            let is_promotion = mv.promotion.is_some();

            let undo = board.make_move(mv);
            let gives_check = board.in_check(board.side);

            // Check extension: langkah yang memberi skak diperpanjang +1 ply -
            // rangkaian skak paksa (sering mengarah ke mat atau kerugian
            // material) diselesaikan lebih dulu, bukan dipotong sembarangan
            // di batas kedalaman iterative deepening. Teknik standar, aman -
            // beda dari singular extension yang butuh Multi-Cut/Negative
            // Extension untuk aman (lihat catatan-riset-referensi.md §4.3).
            let extension = if gives_check { 1 } else { 0 };
            let new_depth = depth - 1 + extension;

            let score = if i == 0 {
                -self.negamax(board, new_depth, -beta, -alpha, ply + 1, true)
            } else {
                // Late Move Reductions: langkah yang diurutkan belakangan
                // (bukan capture/promosi/skak, di depth cukup dalam) dicari
                // dulu di kedalaman tereduksi + jendela sempit (null window,
                // gaya Principal Variation Search). Kalau ternyata bagus
                // (skor > alpha), cari ulang penuh untuk konfirmasi.
                let reduction = if depth >= 3 && i >= 3 && !is_capture && !is_promotion && !gives_check && !in_check {
                    1 + ((i as i32) / 8).min(2)
                } else {
                    0
                };

                let mut s = -self.negamax(board, new_depth - reduction, -alpha - 1, -alpha, ply + 1, true);
                if s > alpha && reduction > 0 {
                    s = -self.negamax(board, new_depth, -alpha - 1, -alpha, ply + 1, true);
                }
                if s > alpha && s < beta {
                    s = -self.negamax(board, new_depth, -beta, -alpha, ply + 1, true);
                }
                s
            };
            board.unmake_move(mv, undo);

            if self.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }
            if best_score > alpha {
                alpha = best_score;
            }
            if alpha >= beta {
                if !is_capture {
                    self.record_killer(ply, mv);
                    self.record_history(mv, depth);
                }
                break;
            }
        }

        let bound = if best_score <= orig_alpha {
            Bound::Upper
        } else if best_score >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt.store(key, best_move, best_score, depth as i8, bound);

        best_score
    }

    fn quiescence(&mut self, board: &mut Board, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        self.check_time();
        if self.stopped {
            return 0;
        }

        let stand_pat = evaluate(board);
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let legal = gen_legal(board);
        let mut captures: Vec<Move> = legal.iter().copied().filter(|m| m.is_capture()).collect();
        sort_by_mvv_lva(board, &mut captures);

        for mv in captures {
            let undo = board.make_move(mv);
            let score = -self.quiescence(board, -beta, -alpha);
            board.unmake_move(mv, undo);

            if self.stopped {
                return 0;
            }
            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }
}

fn mvv_lva_score(board: &Board, mv: &Move) -> i32 {
    let victim_value = if mv.flag == MoveFlag::EnPassant {
        PIECE_VALUE[PAWN]
    } else {
        board.piece_at(mv.to).map(|(_, pt)| PIECE_VALUE[pt]).unwrap_or(0)
    };
    let attacker_value = board.piece_at(mv.from).map(|(_, pt)| PIECE_VALUE[pt]).unwrap_or(0);
    victim_value * 16 - attacker_value
}

fn sort_by_mvv_lva(board: &Board, moves: &mut [Move]) {
    moves.sort_by(|a, b| mvv_lva_score(board, b).cmp(&mvv_lva_score(board, a)));
}

/// Urutan langkah: TT-move, lalu capture (MVV-LVA), lalu 2 killer move, baru
/// quiet move lain diurut skor history.
fn order_moves(
    board: &Board,
    moves: &MoveList,
    tt_move: Option<Move>,
    killers: &[Option<Move>; 2],
    history: &[[i32; 64]; 64],
) -> Vec<Move> {
    let mut scored: Vec<(i32, Move)> = moves
        .iter()
        .map(|&m| {
            let score = if Some(m) == tt_move {
                2_000_000
            } else if m.is_capture() {
                1_000_000 + mvv_lva_score(board, &m)
            } else if Some(m) == killers[0] {
                900_000
            } else if Some(m) == killers[1] {
                800_000
            } else {
                history[m.from as usize][m.to as usize]
            };
            (score, m)
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, m)| m).collect()
}
