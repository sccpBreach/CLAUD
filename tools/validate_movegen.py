#!/usr/bin/env python3
"""Reference move generator - dipakai untuk VALIDASI LOGIKA saja sebelum ditranskripsi ke Rust.
Bukan bagian dari engine final. Prioritas: benar, bukan cepat.
"""

WHITE, BLACK = 0, 1
PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING = 0, 1, 2, 3, 4, 5
MASK64 = (1 << 64) - 1

FILE_A = 0x0101010101010101
FILE_H = 0x8080808080808080
RANK_1 = 0x00000000000000FF
RANK_2 = RANK_1 << 8
RANK_4 = RANK_1 << (8*3)
RANK_5 = RANK_1 << (8*4)
RANK_7 = RANK_1 << (8*6)
RANK_8 = 0xFF00000000000000

def sq(file, rank): return rank * 8 + file
def file_of(s): return s % 8
def rank_of(s): return s // 8
def popcount(bb): return bin(bb).count('1')

def lsb_index(bb): return (bb & -bb).bit_length() - 1

def iter_bits(bb):
    while bb:
        b = bb & (-bb)
        idx = b.bit_length() - 1
        yield idx
        bb ^= b

# --- Precomputed leaper attack tables ---
PAWN_ATTACKS = [[0]*64 for _ in range(2)]
KNIGHT_ATTACKS = [0]*64
KING_ATTACKS = [0]*64

for s in range(64):
    f, r = file_of(s), rank_of(s)
    wa = 0
    for df in (-1, 1):
        nf, nr = f+df, r+1
        if 0 <= nf < 8 and 0 <= nr < 8:
            wa |= 1 << sq(nf, nr)
    PAWN_ATTACKS[WHITE][s] = wa
    ba = 0
    for df in (-1, 1):
        nf, nr = f+df, r-1
        if 0 <= nf < 8 and 0 <= nr < 8:
            ba |= 1 << sq(nf, nr)
    PAWN_ATTACKS[BLACK][s] = ba

    na = 0
    for df, dr in [(1,2),(2,1),(2,-1),(1,-2),(-1,-2),(-2,-1),(-2,1),(-1,2)]:
        nf, nr = f+df, r+dr
        if 0 <= nf < 8 and 0 <= nr < 8:
            na |= 1 << sq(nf, nr)
    KNIGHT_ATTACKS[s] = na

    ka = 0
    for df in (-1,0,1):
        for dr in (-1,0,1):
            if df == 0 and dr == 0: continue
            nf, nr = f+df, r+dr
            if 0 <= nf < 8 and 0 <= nr < 8:
                ka |= 1 << sq(nf, nr)
    KING_ATTACKS[s] = ka

DIRS_BISHOP = [(1,1),(1,-1),(-1,1),(-1,-1)]
DIRS_ROOK = [(1,0),(-1,0),(0,1),(0,-1)]

def sliding_attacks(s, occ, directions):
    attacks = 0
    f0, r0 = file_of(s), rank_of(s)
    for df, dr in directions:
        f, r = f0+df, r0+dr
        while 0 <= f < 8 and 0 <= r < 8:
            t = sq(f, r)
            attacks |= (1 << t)
            if occ & (1 << t):
                break
            f += df
            r += dr
    return attacks

PIECE_CHARS = {PAWN:'p', KNIGHT:'n', BISHOP:'b', ROOK:'r', QUEEN:'q', KING:'k'}

class Move:
    __slots__ = ('frm','to','promo','flag')
    # flag: None, 'ep', 'castle_k', 'castle_q', 'double'
    def __init__(self, frm, to, promo=None, flag=None):
        self.frm, self.to, self.promo, self.flag = frm, to, promo, flag
    def __repr__(self):
        files='abcdefgh'
        s = files[file_of(self.frm)]+str(rank_of(self.frm)+1)+files[file_of(self.to)]+str(rank_of(self.to)+1)
        if self.promo is not None:
            s += PIECE_CHARS[self.promo]
        return s
    def key(self):
        return (self.frm, self.to, self.promo, self.flag)

class Board:
    def __init__(self, fen=None):
        self.set_fen(fen or "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")

    def set_fen(self, fen):
        parts = fen.split()
        self.pieces = [[0]*6 for _ in range(2)]
        rank, file = 7, 0
        pm = {'p':PAWN,'n':KNIGHT,'b':BISHOP,'r':ROOK,'q':QUEEN,'k':KING}
        for c in parts[0]:
            if c == '/':
                rank -= 1; file = 0
            elif c.isdigit():
                file += int(c)
            else:
                color = WHITE if c.isupper() else BLACK
                self.pieces[color][pm[c.lower()]] |= 1 << sq(file, rank)
                file += 1
        self.side = WHITE if parts[1] == 'w' else BLACK
        self.castling = 0
        if 'K' in parts[2]: self.castling |= 1
        if 'Q' in parts[2]: self.castling |= 2
        if 'k' in parts[2]: self.castling |= 4
        if 'q' in parts[2]: self.castling |= 8
        self.ep_square = None
        if len(parts) > 3 and parts[3] != '-':
            self.ep_square = sq(ord(parts[3][0])-ord('a'), int(parts[3][1])-1)
        self.halfmove = int(parts[4]) if len(parts) > 4 else 0
        self.fullmove = int(parts[5]) if len(parts) > 5 else 1

    def occ_color(self, color):
        bb = 0
        for pt in range(6): bb |= self.pieces[color][pt]
        return bb

    def occ(self):
        return self.occ_color(WHITE) | self.occ_color(BLACK)

    def piece_at(self, s):
        bit = 1 << s
        for color in range(2):
            for pt in range(6):
                if self.pieces[color][pt] & bit:
                    return color, pt
        return None

    def attacked_by(self, s, by_color):
        occ = self.occ()
        if PAWN_ATTACKS[1-by_color][s] & self.pieces[by_color][PAWN]: return True
        if KNIGHT_ATTACKS[s] & self.pieces[by_color][KNIGHT]: return True
        if KING_ATTACKS[s] & self.pieces[by_color][KING]: return True
        if sliding_attacks(s, occ, DIRS_BISHOP) & (self.pieces[by_color][BISHOP] | self.pieces[by_color][QUEEN]): return True
        if sliding_attacks(s, occ, DIRS_ROOK) & (self.pieces[by_color][ROOK] | self.pieces[by_color][QUEEN]): return True
        return False

    def king_sq(self, color):
        return lsb_index(self.pieces[color][KING])

    def in_check(self, color):
        return self.attacked_by(self.king_sq(color), 1-color)

    def gen_pseudo_legal(self):
        moves = []
        us, them = self.side, 1-self.side
        own = self.occ_color(us)
        occ = self.occ()
        enemy = self.occ_color(them)

        # Pawns
        pawns = self.pieces[us][PAWN]
        push_dir = 8 if us == WHITE else -8
        start_rank = RANK_2 if us == WHITE else RANK_7
        promo_rank = RANK_8 if us == WHITE else RANK_1
        for frm in iter_bits(pawns):
            to1 = frm + push_dir
            if 0 <= to1 < 64 and not (occ & (1 << to1)):
                if (1 << to1) & promo_rank:
                    for pp in (QUEEN, ROOK, BISHOP, KNIGHT):
                        moves.append(Move(frm, to1, promo=pp))
                else:
                    moves.append(Move(frm, to1))
                    if (1 << frm) & start_rank:
                        to2 = frm + 2*push_dir
                        if 0 <= to2 < 64 and not (occ & (1 << to2)):
                            moves.append(Move(frm, to2, flag='double'))
            for to in iter_bits(PAWN_ATTACKS[us][frm]):
                if enemy & (1 << to):
                    if (1 << to) & promo_rank:
                        for pp in (QUEEN, ROOK, BISHOP, KNIGHT):
                            moves.append(Move(frm, to, promo=pp))
                    else:
                        moves.append(Move(frm, to))
                elif self.ep_square is not None and to == self.ep_square:
                    moves.append(Move(frm, to, flag='ep'))

        # Knights
        for frm in iter_bits(self.pieces[us][KNIGHT]):
            for to in iter_bits(KNIGHT_ATTACKS[frm] & ~own):
                moves.append(Move(frm, to))

        # Bishops / Rooks / Queens
        for pt, dirs in ((BISHOP, DIRS_BISHOP), (ROOK, DIRS_ROOK)):
            for frm in iter_bits(self.pieces[us][pt]):
                for to in iter_bits(sliding_attacks(frm, occ, dirs) & ~own):
                    moves.append(Move(frm, to))
        for frm in iter_bits(self.pieces[us][QUEEN]):
            attacks = sliding_attacks(frm, occ, DIRS_BISHOP) | sliding_attacks(frm, occ, DIRS_ROOK)
            for to in iter_bits(attacks & ~own):
                moves.append(Move(frm, to))

        # King
        ksq = self.king_sq(us)
        for to in iter_bits(KING_ATTACKS[ksq] & ~own):
            moves.append(Move(ksq, to))

        # Castling
        if us == WHITE:
            if self.castling & 1 and not (occ & ((1<<5)|(1<<6))) and not self.in_check(WHITE) \
               and not self.attacked_by(5, BLACK) and not self.attacked_by(6, BLACK):
                moves.append(Move(4, 6, flag='castle_k'))
            if self.castling & 2 and not (occ & ((1<<1)|(1<<2)|(1<<3))) and not self.in_check(WHITE) \
               and not self.attacked_by(3, BLACK) and not self.attacked_by(2, BLACK):
                moves.append(Move(4, 2, flag='castle_q'))
        else:
            if self.castling & 4 and not (occ & ((1<<61)|(1<<62))) and not self.in_check(BLACK) \
               and not self.attacked_by(61, WHITE) and not self.attacked_by(62, WHITE):
                moves.append(Move(60, 62, flag='castle_k'))
            if self.castling & 8 and not (occ & ((1<<57)|(1<<58)|(1<<59))) and not self.in_check(BLACK) \
               and not self.attacked_by(59, WHITE) and not self.attacked_by(58, WHITE):
                moves.append(Move(60, 58, flag='castle_q'))

        return moves

    def make_move(self, m):
        us, them = self.side, 1-self.side
        undo = {
            'castling': self.castling, 'ep_square': self.ep_square,
            'halfmove': self.halfmove, 'captured': None, 'captured_sq': None,
        }
        moved = self.piece_at(m.frm)
        assert moved is not None, f"no piece at {m.frm} for move {m}"
        _, pt = moved

        # remove ep square by default; re-set only for double pushes
        self.ep_square = None

        # handle capture (incl. en passant)
        if m.flag == 'ep':
            cap_sq = m.to - 8 if us == WHITE else m.to + 8
            undo['captured'] = (them, PAWN)
            undo['captured_sq'] = cap_sq
            self.pieces[them][PAWN] &= ~(1 << cap_sq)
        else:
            target = self.piece_at(m.to)
            if target is not None:
                undo['captured'] = target
                undo['captured_sq'] = m.to
                self.pieces[target[0]][target[1]] &= ~(1 << m.to)

        # move the piece
        self.pieces[us][pt] &= ~(1 << m.frm)
        if m.promo is not None:
            self.pieces[us][m.promo] |= (1 << m.to)
        else:
            self.pieces[us][pt] |= (1 << m.to)

        # rook hop for castling
        if m.flag == 'castle_k':
            rook_from, rook_to = (7, 5) if us == WHITE else (63, 61)
            self.pieces[us][ROOK] &= ~(1 << rook_from)
            self.pieces[us][ROOK] |= (1 << rook_to)
        elif m.flag == 'castle_q':
            rook_from, rook_to = (0, 3) if us == WHITE else (56, 59)
            self.pieces[us][ROOK] &= ~(1 << rook_from)
            self.pieces[us][ROOK] |= (1 << rook_to)

        # double push -> set ep square
        if m.flag == 'double':
            self.ep_square = (m.frm + m.to) // 2

        # castling rights update
        for s_, mask in ((4,0b0011) if False else []):
            pass
        def clear_right(square):
            if square == 4: self.castling &= ~0b0011
            elif square == 60: self.castling &= ~0b1100
            elif square == 0: self.castling &= ~0b0010
            elif square == 7: self.castling &= ~0b0001
            elif square == 56: self.castling &= ~0b1000
            elif square == 63: self.castling &= ~0b0100
        clear_right(m.frm)
        clear_right(m.to)

        # halfmove clock
        if pt == PAWN or undo['captured'] is not None:
            self.halfmove = 0
        else:
            self.halfmove += 1

        self.side = them
        if us == BLACK:
            self.fullmove += 1
        return undo

    def unmake_move(self, m, undo):
        them = self.side  # side that just moved has flipped already; 'them' here = mover after we flip back
        us = 1 - them
        self.side = us
        if us == BLACK:
            self.fullmove -= 1
        self.castling = undo['castling']
        self.ep_square = undo['ep_square']
        self.halfmove = undo['halfmove']

        pt = m.promo if m.promo is not None else self.piece_at(m.to)[1]
        # remove piece from 'to'
        placed_pt = m.promo if m.promo is not None else pt
        self.pieces[us][placed_pt] &= ~(1 << m.to)
        # restore mover to 'from' as original piece type (pawn if promo)
        orig_pt = PAWN if m.promo is not None else placed_pt
        self.pieces[us][orig_pt] |= (1 << m.frm)

        if m.flag == 'castle_k':
            rook_from, rook_to = (7, 5) if us == WHITE else (63, 61)
            self.pieces[us][ROOK] &= ~(1 << rook_to)
            self.pieces[us][ROOK] |= (1 << rook_from)
        elif m.flag == 'castle_q':
            rook_from, rook_to = (0, 3) if us == WHITE else (56, 59)
            self.pieces[us][ROOK] &= ~(1 << rook_to)
            self.pieces[us][ROOK] |= (1 << rook_from)

        if undo['captured'] is not None:
            cc, cpt = undo['captured']
            self.pieces[cc][cpt] |= (1 << undo['captured_sq'])

    def gen_legal(self):
        legal = []
        us = self.side
        for m in self.gen_pseudo_legal():
            undo = self.make_move(m)
            if not self.in_check(us):
                legal.append(m)
            self.unmake_move(m, undo)
        return legal

    def perft(self, depth):
        if depth == 0:
            return 1
        total = 0
        for m in self.gen_legal():
            undo = self.make_move(m)
            total += self.perft(depth-1)
            self.unmake_move(m, undo)
        return total

def perft_divide(board, depth):
    result = {}
    for m in board.gen_legal():
        undo = board.make_move(m)
        result[repr(m)] = board.perft(depth-1)
        board.unmake_move(m, undo)
    return result

if __name__ == '__main__':
    TESTS = [
        ("startpos", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
         [1, 20, 400, 8902, 197281, 4865609]),
        ("kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
         [1, 48, 2039, 97862, 4085603]),
        ("pos3", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
         [1, 14, 191, 2812, 43238, 674624]),
        ("pos4", "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
         [1, 6, 264, 9467, 422333]),
        ("pos5", "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
         [1, 44, 1486, 62379, 2103487]),
    ]
    all_ok = True
    for name, fen, expected in TESTS:
        print(f"=== {name} ===")
        b = Board(fen)
        for depth, exp in enumerate(expected):
            got = b.perft(depth)
            status = "OK" if got == exp else "MISMATCH"
            if got != exp: all_ok = False
            print(f"  depth {depth}: expected {exp:>10}  got {got:>10}  [{status}]")
    print()
    print("SEMUA COCOK" if all_ok else "ADA YANG TIDAK COCOK - perlu perbaikan")
