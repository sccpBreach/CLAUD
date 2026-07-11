# STATUS Engine CLAUD — Handoff untuk Claude

## Ringkasan

Engine catur berbasis Rust dengan alpha-beta search, pruning, dan move ordering.
Sudah Fase 1 selesai dan terverifikasi. Belum NNUE (Fase 2).

---

## Riwayat Verifikasi

| Tanggal | Verifikasi | Platform |
|---|---|---|
| Build + perfttest (SEMUA COCOK) + bench (179505 nodes) | ✅ | Windows + Google Colab (Linux) |
| CI GitHub Actions (build + perft + bench) | ✅ | Linux + Windows |
| A/B test: killer/history → **+180 Elo** | ✅ | Windows |

---

## Fitur yang sudah diimplementasikan

### Movegen (Fase 0)
- [x] Pseudo-legal generation + legal filter via make/unmake
- [x] Pawn push/double/promotion/capture/en passant
- [x] Knight, Bishop, Rook, Queen, King moves
- [x] Castling (Kingside + Queenside)
- [x] Perft SEMUA COCOK di 5 posisi standar (depth 4-6)

### Search (Fase 1)
- [x] Negamax + alpha-beta + iterative deepening
- [x] Quiescence search (capture-only)
- [x] Transposition Table (ukuran bisa diatur via `setoption name Hash`)
- [x] Reverse Futility Pruning (depth ≤ 6, margin 80cp/ply)
- [x] Razoring (depth ≤ 3, margin 150+100*(depth-1))
- [x] Null Move Pruning (depth ≥ 3, R=3, cek zugzwang)
- [x] Late Move Reductions + PVS (depth ≥ 3, index ≥ 3)
- [x] Check extension (+1 ply saat memberi skak)
- [x] **Killer move** (2 per ply)
- [x] **History heuristic** [from][to]

### Move Ordering
- [x] TT move (prioritas tertinggi)
- [x] MVV-LVA untuk capture
- [x] Killer move (2 per ply)
- [x] History heuristic (quiet move)

### UCI Protocol
- [x] `uci` / `isready` / `ucinewgame` / `quit`
- [x] `position startpos` / `position fen ...` / `moves ...`
- [x] `go depth` / `movetime` / `wtime` / `btime` / `winc` / `binc`
- [x] `setoption name Hash value N` (1-1024 MB)
- [x] `setoption name Threads` (diterima, diabaikan — single-thread)
- [x] `bench` (format OpenBench: `<N> nodes <M> nps`)
- [x] `perft` / `perfttest`

### Tooling
- [x] Makefile kompatibel OpenBench (`make EXE=engine`)
- [x] `.github/workflows/ci.yml` (build + perft + bench di Linux & Windows)
- [x] `tools/validate_movegen.py` (referensi Python)
- [x] `tools/selfplay.py` (UCI match runner)

---

## Struktur Kode

```
engine/
├── .github/workflows/ci.yml   # CI otomatis
├── Cargo.toml                  # Rust project
├── Makefile                    # OpenBench wrapper
├── README.md                   # Dokumentasi utama
├── SETUP-OPENBENCH.md          # Panduan setup OpenBench
├── STATUS.md                   # ← INI (handoff)
├── src/
│   ├── main.rs                 # Entry point + CLI
│   ├── types.rs                # Konstanta warna/jenis bidak
│   ├── bitboard.rs             # Bitboard wrapper u64
│   ├── attacks.rs              # Generate serangan (ray-casting, belum magic)
│   ├── board.rs                # Board + make/unmake move
│   ├── moves.rs                # Move struct + MoveList
│   ├── movegen.rs              # Pseudo-legal + legal filter
│   ├── perft.rs                # Perft + suite 5 posisi
│   ├── zobrist.rs              # Zobrist hashing (fresh, belum incremental)
│   ├── eval.rs                 # Material + PST (akan diganti NNUE)
│   ├── tt.rs                   # Transposition table (single-thread)
│   ├── search.rs               # Negamax + pruning + ordering
│   └── uci.rs                  # UCI loop + bench
└── tools/
    ├── validate_movegen.py     # Validasi movegen
    └── selfplay.py             # Match runner untuk A/B test
```

---

## Hasil A/B Test (Terakhir)

**Killer/History: +180 Elo**
```
Match: Tanpa Killer/History vs Dengan Killer/History
20 game, 100ms/move, 341 detik

Tanpa Killer/History: 0 menang, 9 kalah, 11 seri
Dengan Killer/History: 9 menang, 0 kalah, 11 seri
Elo Δ = +180 (untuk versi dengan killer/history)
```

**Belum di-test via A/B:**
- RFP/Razoring/NMP/LMR
- Check extension

---

## GitHub

- **Repo**: https://github.com/sccpBreach/CLAUD
- **Branch**: `main`
- **CI**: GitHub Actions (build + perfttest + bench) — cek Actions tab
- **Commit terakhir**: `473968c` — add selfplay test harness

---

## Yang Perlu Dilakukan Selanjutnya

### Prioritas: Setup OpenBench + Worker

1. **Fork OpenBench** — https://github.com/AndyGrant/OpenBench → Fork ke akun sccpBreach
2. **Hosting OpenBench** — pilih salah satu:
   - **PythonAnywhere** (gratis, web-based) — panduan di SETUP-OPENBENCH.md
   - **Oracle Cloud** (gratis, 4 core ARM, 24GB RAM) — butuh SSH
   - **Local** — sudah dicoba setup Django di Windows, terkendala fastchess (match runner)
3. **Worker** — butuh Linux (WSL2 di Windows):
   - Nyalakan Virtualization di BIOS (Intel VT-x)
   - `wsl --install` → install Ubuntu → install Rust → clone CLAUD → setup OpenBench Client
4. **Daftarkan engine CLAUD** di instance OpenBench
5. **Submit SPRT test** — bandingkan commit berbeda

### Langkah Inti yang Belum

- [ ] **NNUE (Fase 2)** — evaluasi neural network ganti eval material+PST
  - Butuh: datagen (self-play massal) + Bullet training pipeline
  - Butuh: worker 24/7 (Oracle Cloud atau WSL2)
- [ ] **Magic bitboards** — akselerasi movegen (Fase 0.5)
- [ ] **Singular Extension** — ekstensi di TT hit (Fase 3)
- [ ] **Lazy SMP** — multi-threading
- [ ] **Manajemen waktu lebih baik**
- [ ] **Zobrist incremental** — ganti fresh compute
- [ ] **History gravity/malus** — kurangi skor move yang gagal cutoff

---

## Catatan Teknis

- **Warnings build**: 8 warning (unused code) — semua sengaja (utility functions untuk optimisasi nanti)
- **Platform**: Sudah terverifikasi di Windows (native) dan Linux (Colab + CI)
- **Perft**: SEMUA COCOK di 5 posisi standar
- **Bench**: 179505 nodes
- **NPS**: ~400k (Windows), ~388k (Colab)
