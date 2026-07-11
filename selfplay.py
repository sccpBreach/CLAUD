"""Match runner UCI sederhana untuk A/B test lokal - versi diperbaiki.

Perubahan dari versi awal:
1. Pembukaan diacak per PASANGAN game (N ply acak dari startpos, warna
   ditukar dalam satu pasangan) - supaya game-game itu genuinely berbeda,
   bukan lintasan deterministik yang sama terulang. Prinsip yang sama
   dipakai Fishtest/OpenBench: "paired games" dengan pembukaan sama tapi
   warna ditukar, supaya keunggulan langkah pertama saling meniadakan.
2. `claim_draw=True` - hormati aturan 50 langkah/repetisi tiga kali sesuai
   standar, jangan tunggu sampai batas otomatis 75 langkah/5 kali.
3. Laporan sekarang menampilkan INTERVAL KEPERCAYAAN (kira-kira 95%), bukan
   cuma titik estimasi Elo - supaya tidak overclaim presisi dari sampel kecil.
4. `--seed` untuk reproduksibilitas.

Tetap: ini alat sanity-check cepat lokal, BUKAN pengganti SPRT OpenBench
sungguhan (yang pakai lebih banyak game + kondisi lebih terkontrol). Anggap
hasil dari sini sebagai "layak dicoba lebih lanjut" atau "kelihatannya tidak
membantu", bukan angka Elo final.
"""
import argparse
import math
import random
import time
from pathlib import Path

import chess
import chess.engine


def random_opening(plies: int, rng: random.Random) -> chess.Board:
    board = chess.Board()
    for _ in range(plies):
        if board.is_game_over(claim_draw=True):
            break
        legal = list(board.legal_moves)
        board.push(rng.choice(legal))
    return board


def play_game(white_path: Path, black_path: Path, time_s: float, start_board: chess.Board) -> str:
    w = chess.engine.SimpleEngine.popen_uci(str(white_path))
    b = chess.engine.SimpleEngine.popen_uci(str(black_path))
    board = start_board.copy()
    limit = chess.engine.Limit(time=time_s)
    try:
        while not board.is_game_over(claim_draw=True):
            engine = w if board.turn == chess.WHITE else b
            result = engine.play(board, limit)
            if result.move is None:
                break
            board.push(result.move)
    finally:
        w.quit()
        b.quit()

    outcome = board.outcome(claim_draw=True)
    if outcome is None or outcome.winner is None:
        return "1/2-1/2"
    return "1-0" if outcome.winner == chess.WHITE else "0-1"


def elo_from_score(score: float) -> float:
    score = min(max(score, 1e-6), 1 - 1e-6)
    return 400 * math.log10(score / (1 - score))


def summarize(w: int, l: int, d: int) -> str:
    n = w + l + d
    if n == 0:
        return "belum ada game selesai"
    score = (w + 0.5 * d) / n
    # Pendekatan standar error skor (asumsi game independen - dalam praktik
    # game berpasangan sedikit berkorelasi, jadi ini estimasi kasar, bukan
    # SPRT yang presisi. Cukup untuk "layak dicoba lanjut" vs "belum jelas".
    se = math.sqrt(max(score * (1 - score), 1e-9) / n)
    lo, hi = score - 1.96 * se, score + 1.96 * se
    elo = elo_from_score(score)
    elo_lo, elo_hi = elo_from_score(lo), elo_from_score(hi)
    return (
        f"W={w} L={l} D={d} N={n}  score={score*100:.1f}%  "
        f"Elo≈{elo:+.0f}  (95% CI kira-kira [{elo_lo:+.0f}, {elo_hi:+.0f}])"
    )


def main():
    ap = argparse.ArgumentParser(description="A/B match runner dengan pembukaan acak + CI")
    ap.add_argument("e1", type=Path)
    ap.add_argument("e2", type=Path, nargs="?")
    ap.add_argument("-n", "--games", type=int, default=200, help="jumlah game (default 200, harus genap)")
    ap.add_argument("-t", "--time", type=float, default=0.1, help="detik per langkah")
    ap.add_argument("--opening-plies", type=int, default=8, help="jumlah ply acak untuk pembukaan tiap pasangan")
    ap.add_argument("--seed", type=int, default=42, help="seed RNG supaya hasil bisa direproduksi")
    args = ap.parse_args()

    p1, p2 = args.e1, args.e2 or args.e1
    n = args.games + (args.games % 2)  # paksa genap - satu pasangan = 2 game warna ditukar
    rng = random.Random(args.seed)

    w = l = d = 0
    t0 = time.time()
    for pair in range(n // 2):
        opening = random_opening(args.opening_plies, rng)

        r1 = play_game(p1, p2, args.time, opening)
        w += r1 == "1-0"; l += r1 == "0-1"; d += r1 == "1/2-1/2"

        r2 = play_game(p2, p1, args.time, opening)
        w += r2 == "0-1"; l += r2 == "1-0"; d += r2 == "1/2-1/2"

        done = (pair + 1) * 2
        elapsed = time.time() - t0
        print(f"[{done}/{n}] {r1} / {r2}  |  {summarize(w, l, d)}  ({elapsed:.0f}s)")

    print()
    print("HASIL AKHIR:", summarize(w, l, d))
    print(
        "Catatan: ini sanity-check lokal dengan pembukaan acak, bukan SPRT resmi. "
        "N besar + CI sempit = sinyal kuat untuk lanjut ke OpenBench; N kecil/CI "
        "lebar (apalagi kalau merentang lewati 0) = belum cukup bukti."
    )


if __name__ == "__main__":
    main()
