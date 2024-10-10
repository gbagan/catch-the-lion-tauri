use std::collections::HashMap;

use lazy_static::lazy_static;

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, serde::Deserialize)]
enum Kind { Chick, Elephant, Giraffe, Lion, Hen }

#[derive(PartialEq, Eq, Clone, Copy, serde::Deserialize)]
pub struct Piece {
    kind: Kind,
    position: u8,
    owner: bool,
}

pub type Pieces = [Piece; 8];

#[derive(Clone, Copy, serde::Serialize)]
pub struct Move {
    from: usize,
    to: usize,
}
#[derive(Clone, Copy, Debug)]
enum Flag { Exact, Alpha, Beta }

// transposition table
type Table = HashMap<u64, (u8, i32, Flag)>;

const PIECE_VALUE: [i32; 5] = [10, 30, 50, 10000, 70];

lazy_static! {
    static ref MOVE_DICT: [Vec<[i8; 2]>; 5] = [
        vec![[0, 1]],  // chick
        vec![[1, 1], [-1, 1], [1, -1], [-1, -1]], // elephant
        vec![[0, 1], [1, 0], [0, -1], [-1, 0]], // giraffe
        vec![[0, 1], [1, 0], [0, -1], [-1, 0], [1, 1], [-1, 1], [1, -1], [-1, -1]], // lion
        vec![[0, 1], [1, 0], [0, -1], [-1, 0], [1, 1], [-1, 1]] // hen
    ];
}

fn encode_pieces(pieces: &Pieces, turn: bool) -> u64 {
    let mut encoding = 0;
    for piece in pieces {
        encoding |= piece.position as u64;
        if piece.owner {
            encoding |= 16;
        }
        encoding <<= 5;
    }
    if turn {
        encoding |= 1;
    }
    if pieces[3].kind == Kind::Hen {
        encoding |= 2;
    }
    if pieces[7].kind == Kind::Hen {
        encoding |= 4;
    }
    encoding
}

fn possible_moves(pieces: &Pieces, turn: bool) -> Vec<Move> {
    let mut result = vec![];
    let mut board = [0u8; 12];
    for piece in pieces {
        if piece.position < 12 {
            board[piece.position as usize] = if piece.owner { 2 } else { 1 };
        }
    }

    for (i, &piece) in pieces.iter().enumerate() {
        if piece.owner != turn {
            continue;
        }
        if piece.position == 12 {
            if i < 4 || pieces[i - 4].owner != piece.owner || pieces[i - 4].position < 12 {
                for j in 0..12 {
                    if board[j] == 0 {
                        result.push(Move {from: i, to: j})
                    }
                }
            }
        } else {
            let x = piece.position as i8 % 3;
            let y = piece.position as i8 / 3;
            let owner = if piece.owner { 2 } else { 1 };
            for &[dx, dy] in &MOVE_DICT[piece.kind as usize] {
                let [dx, dy] = if turn { [dx, dy] } else { [-dx, -dy] };
                let x2 = x + dx;
                let y2 = y + dy;
                if x2 >= 0 && x2 < 3 && y2 >= 0 && y2 < 4 {
                    let index = (3 * y2 + x2) as usize;
                    if board[index] != owner {
                        result.push(Move {from: i, to: index})
                    }
                }
            }
        }
    }
    result
}

fn play_move(pieces: &Pieces, mov: Move) -> Pieces {
    let to = mov.to as u8;
    let Piece {owner, kind, position} = pieces[mov.from as usize];
    let mut new_pieces = *pieces;

    if let Some(j) = pieces.iter().position(|p| p.position == to) {
        new_pieces[j].position = 12;
        new_pieces[j].owner = owner;
        if new_pieces[j].kind == Kind::Hen {
            new_pieces[j].kind = Kind::Chick;
        }  
    }
    new_pieces[mov.from].position = to;
    if kind == Kind::Chick && position < 12 && (owner && to > 8 || !owner && to < 3) {
        new_pieces[mov.from].kind = Kind::Hen;
    }
    new_pieces
}

fn evaluate_position(pieces: &Pieces) -> i32 {
    let mut result = 0;
  
    let mut board = [0u8; 12];
    for piece in pieces {
        result += (if piece.owner {-1} else {1}) * PIECE_VALUE[piece.kind as usize];
        if piece.position < 12 {
            board[piece.position as usize] = if piece.owner { 2 } else { 1 };
        }
    }

    for piece in pieces {
        if piece.position < 12 {
            let owner = if piece.owner {2} else {1};
            let dscore = if piece.owner {-1} else {1};
            let x = piece.position as i8 % 3;
            let y = piece.position as i8 / 3;
            for &[dx, dy] in &MOVE_DICT[piece.kind as usize] {
                let [dx, dy] = if piece.owner { [dx, dy] } else { [-dx, -dy] };
                let x2 = x + dx;
                let y2 = y + dy;
                if x2 >= 0 && x2 < 3 && y2 >= 0 && y2 < 4 {
                    let index = (3 * y2 + x2) as usize;
                    if board[index] != owner {
                        result += dscore;
                    }
                }
            }
        }
    }
    result
}

fn alphabeta(table: &mut Table, depth: u8, turn: bool, mut alpha: i32, mut beta: i32, pieces: Pieces) -> i32 {
    let encoding = encode_pieces(&pieces, turn);
    let alpha_orig = alpha;
    let beta_orig = beta;
    if let Some(&(depth2, score, flag)) = table.get(&encoding) {
        if depth2 == depth {
            match flag {
                Flag::Exact => return score,
                Flag::Alpha => alpha = alpha.max(score),
                Flag::Beta => beta = beta.min(score),
            }
        }
        if alpha >= beta {
            return score;
        }
    }
    if depth == 0 {
        return evaluate_position(&pieces)
    } else if pieces[1].position == 12 { // white Lion has been captured
        return -100000-(depth as i32)
    } else if pieces[5].position == 12 { // black Lion has been captured
        return 100000+(depth as i32)
    } else if turn && pieces[5].position > 8 { // black Lion has reached the enemy camp
        return -100000-(depth as i32)
    } else if !turn && pieces[1].position < 3 { // white Lion has reached the enemy camp
        return 100000+(depth as i32)
    }
    
    if !turn {  // maximizing
        let mut best_score = i32::MIN;
        for mov in possible_moves(&pieces, turn) {
            let new_pieces = play_move(&pieces, mov);
            let score = alphabeta(table, depth - 1, true, alpha, beta, new_pieces);
            best_score = best_score.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break
            }
        }
        let flag = 
            if best_score <= alpha_orig {
                Flag::Beta
            } else if best_score >= beta{
                Flag::Alpha
            } else {
                Flag::Exact
            };
        table.insert(encoding, (depth, best_score, flag));
        alpha
    } else {   // minimizing
        let mut best_score = i32::MAX;
        for mov in possible_moves(&pieces, turn) {
            let new_pieces = play_move(&pieces, mov);
            let score = alphabeta(table, depth - 1, false, alpha, beta, new_pieces);
            best_score = best_score.min(score);
            beta = beta.min(score);
            if alpha >= beta {
                break
            }
        }
        let flag = 
        if best_score >= beta_orig {
            Flag::Alpha
        } else if best_score <= alpha {
            Flag::Beta
        } else {
            Flag::Exact
        };
        table.insert(encoding, (depth, best_score, flag));
        beta
    }
}


#[tauri::command(async)]
pub fn shogi_ai(pieces: Pieces, played: Vec<Pieces>, depth: u8, turn: bool) -> Move {
    let mut alpha = i32::MIN;
    let mut beta = i32::MAX;
    let mut table: Table = HashMap::new();

    let (played_twice, not_played_twice): (Vec<_>, Vec<_>) =
        possible_moves(&pieces, turn)
            .iter()
            .map(|mov| (*mov, play_move(&pieces, *mov)))
            .partition(|(_, pieces)|
                played.iter().filter(|&ps| ps == pieces).count() >= 1
            );

    let mut best_move = None;
    for (mov, new_pieces) in not_played_twice {
        if !turn {
            let score = alphabeta(&mut table, depth - 1, true, alpha, beta, new_pieces);
            if score > alpha {
                alpha = score;
                best_move = Some(mov);
            }
        } else {
            let score = alphabeta(&mut table, depth - 1, false, alpha, beta, new_pieces);
            if score < beta {
                beta = score;
                best_move = Some(mov);
            }
        }
    }
    if let Some(mov) = best_move { //&& (if turn {beta <= 0} else {alpha >= 0})
        return mov;
    }
  
    for (mov, new_pieces) in played_twice {
        if !turn {
            let score = alphabeta(&mut table, depth - 1, true, alpha, beta, new_pieces);
            if score > alpha {
                alpha = score;
                best_move = Some(mov);
            }
        } else {
            let score = alphabeta(&mut table, depth - 1, false, alpha, beta, new_pieces);
            if score < beta {
                beta = score;
                best_move = Some(mov);
            }
        }
    }
    best_move.unwrap()
}