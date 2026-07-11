import subprocess, sys, time, argparse, random, math
from pathlib import Path
import chess, chess.engine

OPENING_PLY = 8

def random_opening() -> chess.Board:
    board = chess.Board()
    for _ in range(OPENING_PLY):
        legal = list(board.legal_moves)
        if not legal:
            break
        board.push(random.choice(legal))
    return board

def game(white_path, black_path, time_s, opening: chess.Board):
    w = chess.engine.SimpleEngine.popen_uci(str(white_path))
    b = chess.engine.SimpleEngine.popen_uci(str(black_path))
    board = opening.copy()
    limit = chess.engine.Limit(time=time_s)
    while not board.is_game_over():
        e = w if board.turn == chess.WHITE else b
        r = e.play(board, limit)
        board.push(r.move)
    w.quit(); b.quit()
    o = board.outcome()
    if o is None or o.winner is None:
        return "1/2-1/2"
    return "1-0" if o.winner == chess.WHITE else "0-1"

def elo_ci(wins, losses, draws):
    n = wins + losses + draws
    if n == 0:
        return (0, 0, 0)
    score = (wins + 0.5 * draws) / n
    mu = (score - 0.5) * 400 / math.log(10)
    p = score
    z = 1.96
    margin = z * math.sqrt(p * (1 - p) / n) if n > 0 else 0
    lo = (p - margin - 0.5) * 400 / math.log(10) if p - margin > 0 else -999
    hi = (p + margin - 0.5) * 400 / math.log(10) if p + margin < 1 else 999
    return (mu, lo, hi)

def main():
    ap = argparse.ArgumentParser(description="UCI match runner with random openings + CI")
    ap.add_argument("e1", type=Path, help="Engine 1 (plays white in odd games)")
    ap.add_argument("e2", type=Path, nargs="?", help="Engine 2 (defaults to e1)")
    ap.add_argument("-n", type=int, default=200, help="Number of games (default 200)")
    ap.add_argument("-t", type=float, default=0.1, help="Time per move in seconds (default 0.1)")
    a = ap.parse_args()
    p1, p2 = a.e1, a.e2 or a.e1
    if a.n % 2: a.n += 1
    w, l, d, t0 = 0, 0, 0, time.time()
    random.seed()
    for i in range(0, a.n, 2):
        opening = random_opening()
        r1 = game(p1, p2, a.t, opening)
        w += r1 == "1-0"; l += r1 == "0-1"; d += r1 == "1/2-1/2"
        print(f"G{i+1}: {r1}")
        opening2 = random_opening()
        r2 = game(p2, p1, a.t, opening2)
        w += r2 == "0-1"; l += r2 == "1-0"; d += r2 == "1/2-1/2"
        print(f"G{i+2}: {r2}")
        el = time.time() - t0
        rate = (i+2)/el if el > 0 else 0
        print(f"  [{i+2}/{a.n}] W={w} L={l} D={d}  {el:.0f}s ({rate:.1f}g/s)")
    el = time.time() - t0
    total = w + l + d
    mu, lo, hi = elo_ci(w, l, d)
    print(f"\n{'='*60}")
    print(f"MATCH RESULT ({a.n}g {a.t}s/move)")
    print(f"  {p1} vs {p2}")
    print(f"  Wins: {w} ({100*w/total:.1f}%)  Losses: {l} ({100*l/total:.1f}%)  Draws: {d} ({100*d/total:.1f}%)")
    print(f"  Elo: {mu:+.0f}  95%CI: [{lo:+.0f}, {hi:+.0f}]")
    print(f"  Time: {el:.0f}s ({total/el:.1f}g/s)")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
