# Panduan Setup OpenBench

Ini bagian yang tidak bisa saya kerjakan langsung (butuh akun, hosting, dan
keputusan yang harus kamu ambil sendiri) - tapi ini peta lengkapnya supaya
tidak meraba-raba.

## Kenapa ini penting sekarang

Semua yang sudah dibangun (RFP, Razoring, NMP, LMR, killer move, history
heuristic, check extension) baru terbukti **"tidak merusak"** - belum ada
satupun yang terbukti **"beneran menaikkan kekuatan"**. Satu-satunya cara
membuktikan itu adalah SPRT: ribuan game self-play antara versi lama vs versi
baru, dianalisis statistik. OpenBench adalah kerangka kerja yang menjalankan
ini secara terdistribusi.

## Status kesiapan engine kita

Sudah siap dari sisi kode:
- [x] `Makefile` dengan dukungan `EXE=` (pembungkus `cargo build`)
- [x] Command `bench` format `<N> nodes <M> nps`
- [x] UCI option `Hash` fungsional (`setoption name Hash value N`)
- [x] UCI option `Threads` dideklarasikan (single-thread, OpenBench tetap bisa uji ini)
- [x] Git repo + `.gitignore` + CI (`.github/workflows/ci.yml`)

Belum ada, dan jadi tanggung jawabmu:
- [ ] Repo di GitHub (push kode ini ke sana)
- [ ] Instance OpenBench (pilih salah satu opsi di bawah)
- [ ] Minimal satu worker (mesin yang benar-benar menjalankan game)

## Langkah 1: Push ke GitHub

```bash
cd engine
git init
git add .
git commit -m "Fase 0+1: movegen, alpha-beta, pruning inti, UCI"
# buat repo baru di github.com, lalu:
git remote add origin https://github.com/<username>/<nama-repo>.git
git branch -M main
git push -u origin main
```

Cek CI otomatis jalan di tab "Actions" repo-mu - kalau hijau, berarti build +
perft + bench semuanya lolos di server GitHub (Linux DAN Windows sekaligus,
independen dari mesinmu sendiri). Ini konfirmasi ekstra yang bagus.

## Langkah 2: Pilih instance OpenBench

**Opsi A - Self-host di PythonAnywhere (gratis, direkomendasikan untuk mulai):**
1. Fork `AndyGrant/OpenBench` di GitHub.
2. Daftar akun gratis di pythonanywhere.com.
3. Ikuti panduan setup di wiki resmi OpenBench (`github.com/AndyGrant/OpenBench/wiki`) -
   intinya: clone fork-mu di PythonAnywhere, setup Django + database, konfigurasi
   web app.
4. Kamu jadi admin instance sendiri - bisa tambah engine kamu sendiri kapan pun,
   tidak perlu izin siapa pun.

**Opsi B - Minta ditambahkan ke instance publik yang sudah ada:**
Beberapa instance publik (misalnya yang dipakai Berserk/Ethereal) kadang
menerima engine baru dari komunitas. Ini lebih cepat tapi bergantung
keputusan admin instance itu, dan enginemu jadi "tamu" bukan milik sendiri.
Untuk belajar/eksperimen bebas, Opsi A lebih fleksibel.

**Rekomendasi saya: mulai dari Opsi A.** Kontrol penuh, tidak perlu approval,
dan kalau nanti enginemu sudah cukup matang untuk publik, gampang pindah/gabung
ke instance komunitas.

## Langkah 3: Siapkan worker

Worker adalah mesin yang benar-benar compile + jalankan game. Rekomendasi:
**Linux (native atau WSL2 di Windows-mu yang sekarang)** - alasan:
- OpenBench worker script (`OpenBench/Client`) paling teruji di Linux.
- Build lintas-platform lebih konsisten (menghindari isu path/permission ala Windows).
- Selaras dengan target akhir (TCEC mensyaratkan Linux untuk submisi).

Cara cepat dapat Linux di mesin Windows-mu sekarang:
```powershell
wsl --install
```
lalu install Rust di dalam WSL (`curl https://sh.rustup.rs -sSf | sh`), clone
repo-mu di sana, dan jalankan worker client OpenBench dari situ.

## Langkah 4: Daftarkan engine & jalankan test pertama

Setelah instance OpenBench jalan dan worker terhubung:
1. Login sebagai admin, tambah "Engine" baru: isi repo GitHub-mu, branch, dan
   command bench (`make` lalu jalankan `bench`, formatnya sudah kita siapkan).
2. OpenBench akan compile engine-mu sekali untuk dapat "bench signature"
   (jumlah node dari `bench` di commit itu) - dipakai memverifikasi semua
   worker build versi yang identik.
3. Submit test pertama: bandingkan commit SEBELUM killer/history vs SESUDAH
   (kita punya riwayat commit yang jelas untuk ini kalau kamu commit tiap
   tahap seperti saran di atas) - SPRT akan mulai jalan begitu worker
   mengambil pekerjaan.

## Setelah ini jalan

Baru dari sini margin-margin RFP/Razoring/NMP/LMR yang masih konservatif bisa
mulai di-tuning berdasarkan bukti statistik, bukan tebakan - dan baru dari
sini juga masuk akal mulai datagen untuk NNUE (Fase 2), karena ada tempat
memvalidasi apakah NNUE beneran lebih baik dari eval material+PST sekarang.
