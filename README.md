# Engine Catur - Fase 0 + Fase 1 (Board/Movegen/UCI + Search + Move Ordering + Hash Option)

Status: **teruji compile dan jalan tiga kali di mesin user** (Windows), semua
lolos. Baru saja ditambah: check extension (perpanjang +1 ply saat memberi
skak) dan `setoption name Hash value N` (TT sekarang beneran bisa diatur
ukurannya - prasyarat OpenBench). Belum ada singular extension - itu Fase 3.

## Riwayat verifikasi (baca dulu sebelum lanjut)

1. **Movegen (Fase 0)** → dikonfirmasi user: compile sukses + `perfttest` SEMUA COCOK.
2. **Alpha-beta polos + TT + qsearch** → dikonfirmasi user: `bench` jalan, node & PV masuk akal.
3. **RFP + Razoring + NMP + LMR** → dikonfirmasi user: node turun 5.2x, perft tetap cocok.
   Satu bug (`fn negamax` dobel) ketahuan & diperbaiki via cek mekanis sebelum diserahkan.
4. **Killer move + history heuristic** → dikonfirmasi user: node startpos turun lagi
   3.7x (killer/history paling berpengaruh di posisi quiet), kiwipete/pos4 nyaris
   tak berubah (wajar - posisi taktis padat capture, MVV-LVA sudah dominan di sana).
5. **Check extension + setoption Hash (baru saja ditambahkan)**: validasi
   struktural saja (extension self-limiting karena rangkaian skak pasti berakhir;
   `Hash` value di-clamp 1-1024 MB). **BELUM dikonfirmasi compile.**

**Catatan jujur soal proses lagi**: waktu menyisipkan `handle_setoption`, saya
sempat SALAH melakukan `str_replace` dan tidak sengaja menghapus satu baris
kode (`if uci.len() < 4 {`) di fungsi lain, yang akan membuat kode tidak valid
kalau tidak ketahuan. Ketahuan lagi lewat pengecekan mekanis (kurung tidak
seimbang) sebelum sempat diserahkan, dan sudah diperbaiki. Pola yang sama
seperti bug `fn negamax` dobel sebelumnya - alasan kenapa saya selalu
menjalankan cek kurung/duplikasi fungsi sebelum mengemas kode, dan kenapa
verifikasi compile di pihakmu tetap penting walau saya sudah cek sendiri.

**WAJIB dijalankan sebelum lanjut fase berikutnya:**
```bash
cargo build --release
./target/release/engine perfttest   # harus tetap "SEMUA COCOK"
./target/release/engine bench       # cek tidak crash, angka node wajar

# Uji setoption baru:
./target/release/engine
setoption name Hash value 64
isready
position startpos
go movetime 2000
quit
```

## Cara build & test

```bash
cargo build --release
./target/release/engine perfttest
./target/release/engine bench

./target/release/engine
uci
isready
position startpos
go movetime 3000
d
quit
```

## Build via Makefile (buat OpenBench nanti)

```bash
make EXE=engine_test
./engine_test bench
```

## Struktur kode

| File | Isi |
|---|---|
| `src/types.rs` | Konstanta warna/jenis bidak, tipe `Square`, helper file/rank |
| `src/bitboard.rs` | Tipe `Bitboard` (wrapper u64) + operasi bit |
| `src/attacks.rs` | Generate serangan tiap bidak (leaper dihitung langsung, slider via ray-casting - **belum magic bitboards**) |
| `src/board.rs` | Struct `Board`, parsing FEN, `make_move`/`unmake_move`/`make_null_move` (titik integrasi NNUE masa depan ditandai komentar `NNUE-HOOK`) |
| `src/moves.rs` | Tipe `Move` (field eksplisit) dan `MoveList` |
| `src/movegen.rs` | Pseudo-legal generation + filter legal |
| `src/perft.rs` | Perft + perft-divide + suite 5 posisi standar |
| `src/zobrist.rs` | Zobrist key untuk TT, dihitung fresh tiap panggilan |
| `src/eval.rs` | Material + PST kecil - placeholder sampai NNUE Fase 2 |
| `src/tt.rs` | Transposition table single-threaded (belum atomic) |
| `src/search.rs` | Negamax + alpha-beta + qsearch + iterative deepening + RFP + Razoring + NMP + LMR/PVS + check extension + killer move + history heuristic + MVV-LVA |
| `src/uci.rs` | Loop UCI + `bench` + `setoption` (Hash) + manajemen waktu dasar |
| `tools/validate_movegen.py` | Referensi Python untuk validasi movegen |

## Keputusan desain yang sengaja disederhanakan (lihat `desain-arsitektur-engine.md` untuk rencana penuh)

- **Movegen**: pseudo-legal+filter, bukan pin/checker-bitboard - optimisasi lanjutan.
- **Attacks**: ray-casting, bukan magic bitboards - optimisasi Fase 0.5.
- **Move**: struct eksplisit, bukan encoding u16 - optimisasi lanjutan.
- **TT belum atomic** - belum ada threading (Lazy SMP), jadi belum ada race
  condition yang perlu dicegah. Upgrade ke `AtomicU64`+Relaxed adalah
  pekerjaan terpisah saat threading mulai dibangun.
- **Zobrist key fresh**, bukan incremental.
- **Skor mate belum disesuaikan lewat TT** (mate-score adjustment).
- **Manajemen waktu sangat sederhana** (1/30 sisa waktu + separuh increment) -
  bukan soft/hard-bound+voting dari rencana desain final (butuh threading dulu).
- **History belum ada "gravity"/malus** untuk move yang dicoba tapi gagal
  cutoff - cuma nambah skor saat cutoff berhasil.
- **RFP/Razoring/NMP/LMR/killer/history** semua pakai parameter tetap yang
  konservatif - belum di-tuning SPSA. Perlu SPRT via OpenBench untuk
  memvalidasi ini beneran net-positive buat kekuatan.
- **Eval** cuma material + PST kecil - akan DIGANTI NNUE di Fase 2.
- **`setoption`** cuma `Hash` yang benar-benar berpengaruh (resize TT).
  `Threads` diterima tapi diabaikan (wajar untuk engine single-thread Fase 1).

## Status terbaru

✅ **A/B test selesai**: Killer move + History heuristic = **+180 Elo** (20 game, 100ms/move).
Lihat `STATUS.md` untuk handoff lengkap ke Claude.

## Langkah selanjutnya

1. **Setup OpenBench** — lihat `SETUP-OPENBENCH.md`
2. **Validasi RFP/Razoring/NMP/LMR** lewat A/B test atau SPRT
3. **NNUE (Fase 2)** — datagen + Bullet pipeline (ganti eval material+PST)
