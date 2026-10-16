use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const W: i32 = 10;
pub const H: i32 = 20;

type Board = [[u8; 10]; 20];

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Left,
    Right,
    Soft,
    Rotate,
    Hard,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

const KINDS: [Kind; 7] = [Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L];

impl Kind {
    fn id(self) -> u8 {
        self as u8 + 1
    }

    fn base(self) -> (i32, [(i32, i32); 4]) {
        match self {
            Kind::I => (4, [(1, 0), (1, 1), (1, 2), (1, 3)]),
            Kind::O => (2, [(0, 0), (0, 1), (1, 0), (1, 1)]),
            Kind::T => (3, [(0, 1), (1, 0), (1, 1), (1, 2)]),
            Kind::S => (3, [(0, 1), (0, 2), (1, 0), (1, 1)]),
            Kind::Z => (3, [(0, 0), (0, 1), (1, 1), (1, 2)]),
            Kind::J => (3, [(0, 0), (1, 0), (1, 1), (1, 2)]),
            Kind::L => (3, [(0, 2), (1, 0), (1, 1), (1, 2)]),
        }
    }
}

#[derive(Clone, Copy)]
struct Piece {
    kind: Kind,
    n: i32,
    cells: [(i32, i32); 4],
    x: i32,
    y: i32,
}

impl Piece {
    fn new(kind: Kind) -> Self {
        let (n, cells) = kind.base();
        Piece {
            kind,
            n,
            cells,
            x: (W - n) / 2,
            y: 0,
        }
    }

    fn rotated(&self) -> Self {
        let n = self.n;
        Piece {
            cells: self.cells.map(|(r, c)| (c, n - 1 - r)),
            ..*self
        }
    }

    fn abs(&self) -> [(i32, i32); 4] {
        self.cells.map(|(r, c)| (self.x + c, self.y + r))
    }
}

fn collides(board: &Board, p: &Piece) -> bool {
    p.abs()
        .iter()
        .any(|&(x, y)| x < 0 || x >= W || y >= H || (y >= 0 && board[y as usize][x as usize] != 0))
}

#[derive(Serialize)]
pub struct View {
    kind: u8,
    cells: [[i32; 2]; 4],
}

fn view(p: &Piece) -> View {
    View {
        kind: p.kind.id(),
        cells: p.abs().map(|(x, y)| [x, y]),
    }
}

fn preview(kind: Kind) -> View {
    let p = Piece::new(kind);
    let minr = p.cells.iter().map(|c| c.0).min().unwrap();
    let minc = p.cells.iter().map(|c| c.1).min().unwrap();
    View {
        kind: kind.id(),
        cells: p.cells.map(|(r, c)| [c - minc, r - minr]),
    }
}

pub struct Game {
    board: Board,
    cur: Piece,
    next: Kind,
    bag: Vec<Kind>,
    hold: Option<Kind>,
    can_hold: bool,
    score: u32,
    lines: u32,
    over: bool,
}

#[derive(Serialize)]
pub struct Snapshot {
    board: Board,
    piece: View,
    ghost: View,
    next: View,
    hold: Option<View>,
    score: u32,
    lines: u32,
    level: u32,
    over: bool,
}

impl Game {
    pub fn new() -> Self {
        let mut g = Game {
            board: [[0; 10]; 20],
            cur: Piece::new(Kind::I),
            next: Kind::I,
            bag: Vec::new(),
            hold: None,
            can_hold: true,
            score: 0,
            lines: 0,
            over: false,
        };
        let first = g.draw();
        g.cur = Piece::new(first);
        g.next = g.draw();
        g
    }

    fn draw(&mut self) -> Kind {
        if self.bag.is_empty() {
            self.bag = KINDS.to_vec();
            self.bag.shuffle(&mut rand::rng());
        }
        self.bag.pop().unwrap()
    }

    fn level(&self) -> u32 {
        self.lines / 10 + 1
    }

    pub fn drop_interval(&self) -> Duration {
        let ms = 800u64
            .saturating_sub(70 * (self.level() as u64 - 1))
            .max(80);
        Duration::from_millis(ms)
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let p = Piece {
            x: self.cur.x + dx,
            y: self.cur.y + dy,
            ..self.cur
        };
        if collides(&self.board, &p) {
            false
        } else {
            self.cur = p;
            true
        }
    }

    fn try_rotate(&mut self) {
        let r = self.cur.rotated();
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0)] {
            let p = Piece {
                x: r.x + dx,
                y: r.y + dy,
                ..r
            };
            if !collides(&self.board, &p) {
                self.cur = p;
                return;
            }
        }
    }

    pub fn apply(&mut self, action: Action) {
        if self.over {
            return;
        }
        match action {
            Action::Left => {
                self.try_move(-1, 0);
            }
            Action::Right => {
                self.try_move(1, 0);
            }
            Action::Soft => {
                if self.try_move(0, 1) {
                    self.score += 1;
                }
            }
            Action::Rotate => self.try_rotate(),
            Action::Hard => {
                while self.try_move(0, 1) {
                    self.score += 2;
                }
                self.lock();
            }
            Action::Hold => self.hold(),
        }
    }

    pub fn gravity(&mut self) {
        if self.over {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock();
        }
    }

    fn lock(&mut self) {
        let mut above_board = false;
        for (x, y) in self.cur.abs() {
            if y < 0 {
                above_board = true;
            } else {
                self.board[y as usize][x as usize] = self.cur.kind.id();
            }
        }
        if above_board {
            self.over = true;
            return;
        }
        self.clear_lines();
        self.can_hold = true;
        self.spawn_next();
    }

    fn clear_lines(&mut self) {
        let kept: Vec<[u8; 10]> = self
            .board
            .iter()
            .copied()
            .filter(|row| row.contains(&0))
            .collect();
        let cleared = 20 - kept.len();
        if cleared == 0 {
            return;
        }
        let level = self.level();
        let mut board = [[0u8; 10]; 20];
        for (i, row) in kept.into_iter().enumerate() {
            board[cleared + i] = row;
        }
        self.board = board;
        self.lines += cleared as u32;
        self.score += [0, 100, 300, 500, 800][cleared] * level;
    }

    fn spawn_next(&mut self) {
        let k = self.next;
        self.next = self.draw();
        self.cur = Piece::new(k);
        if collides(&self.board, &self.cur) {
            self.over = true;
        }
    }

    fn hold(&mut self) {
        if !self.can_hold {
            return;
        }
        let current = self.cur.kind;
        match self.hold.replace(current) {
            Some(k) => self.cur = Piece::new(k),
            None => self.spawn_next(),
        }
        self.can_hold = false;
        if collides(&self.board, &self.cur) {
            self.over = true;
        }
    }

    fn ghost(&self) -> Piece {
        let mut g = self.cur;
        loop {
            let p = Piece { y: g.y + 1, ..g };
            if collides(&self.board, &p) {
                break;
            }
            g = p;
        }
        g
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            board: self.board,
            piece: view(&self.cur),
            ghost: view(&self.ghost()),
            next: preview(self.next),
            hold: self.hold.map(preview),
            score: self.score,
            lines: self.lines,
            level: self.level(),
            over: self.over,
        }
    }
}
