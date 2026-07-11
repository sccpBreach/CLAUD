//! Transposition table.
//!
//! Catatan desain PENTING: ini versi single-threaded (plain, bukan atomic).
//! Rencana arsitektur kita bilang TT akhir harus `AtomicU64`+Relaxed supaya
//! aman dipakai banyak thread (Lazy SMP) tanpa lock. TAPI threading belum ada
//! di Fase 1 ini - baru satu thread yang jalan, jadi belum ada race condition
//! yang perlu dicegah. Menulis versi atomic sekarang, tanpa compiler untuk
//! memverifikasi bit-packing-nya, cuma menambah risiko tanpa manfaat nyata hari
//! ini. Upgrade ke AtomicU64 adalah pekerjaan terpisah saat Lazy SMP mulai
//! dibangun - struct ini sengaja dibuat supaya perubahan itu terkurung rapi di
//! file ini saja.

use crate::moves::Move;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: i8,
    pub bound: Bound,
}

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>().max(1);
        let target_entries = (size_mb * 1024 * 1024) / entry_size;

        let mut num_entries: usize = 1024; // batas bawah wajar
        while num_entries.saturating_mul(2) <= target_entries {
            num_entries *= 2;
        }

        TranspositionTable { entries: vec![None; num_entries], mask: num_entries - 1 }
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & self.mask
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        match self.entries[self.index(key)] {
            Some(e) if e.key == key => Some(e),
            _ => None,
        }
    }

    pub fn store(&mut self, key: u64, best_move: Option<Move>, score: i32, depth: i8, bound: Bound) {
        let idx = self.index(key);
        let should_replace = match self.entries[idx] {
            None => true,
            Some(existing) => existing.key != key || depth >= existing.depth,
        };
        if should_replace {
            self.entries[idx] = Some(TTEntry { key, best_move, score, depth, bound });
        }
    }

    pub fn clear(&mut self) {
        for slot in self.entries.iter_mut() {
            *slot = None;
        }
    }
}
