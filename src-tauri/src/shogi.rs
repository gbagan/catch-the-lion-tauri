use lazy_static::lazy_static;

#[derive(PartialEq, Eq, Clone, Copy, Debug, serde::Deserialize)]
pub struct Piece {
    kind: u8,
    position: i8,
    owner: bool,
}

pub type Pieces = [Piece; 8];

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Move {
    from: usize,
    to: usize,
}

const PIECE_VALUE: [i32; 5] = [10, 30, 50, 1000, 70];

lazy_static! {
    static ref MOVE_DICT: [Vec<[i8; 2]>; 5] = [
        vec![[0, 1]],  // chick
        vec![[1, 1], [-1, 1], [1, -1], [-1, -1]], // elephant
        vec![[0, 1], [1, 0], [0, -1], [-1, 0]], // giraffe
        vec![[0, 1], [1, 0], [0, -1], [-1, 0], [1, 1], [-1, 1], [1, -1], [-1, -1]], // lion
        vec![[0, 1], [1, 0], [0, -1], [-1, 0], [1, 1], [-1, 1]] // hen
    ];
}

fn possible_moves(pieces: &Pieces, turn: bool) -> Vec<Move> {
    let mut result = vec![];
    let mut board = [0u8; 12];
    for piece in pieces {
        if piece.position >= 0 {
            board[piece.position as usize] = if piece.owner { 2 } else { 1 };
        }
    }

    for (i, &piece) in pieces.iter().enumerate() {
        if piece.owner != turn {
            continue;
        }
        if piece.position == -1 {
            if i < 4 || pieces[i - 4].owner != piece.owner || pieces[i - 4].position >= 0 {
                for j in 0..12 {
                    if board[j] == 0 {
                        result.push(Move {from: i, to: j})
                    }
                }
            }
        } else {
            let x = piece.position % 3;
            let y = piece.position / 3;
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
    let to = mov.to as i8;
    let Piece {owner, kind, position} = pieces[mov.from as usize];
    let mut new_pieces = *pieces;

    if let Some(j) = pieces.iter().position(|p| p.position == to) {
        new_pieces[j].position = -1;
        new_pieces[j].owner = owner;
        if new_pieces[j].kind == 4 { // Hen
            new_pieces[j].kind = 0  // Chick
        }  
    }
    new_pieces[mov.from].position = to;
    if kind == 0 && position >= 0 && (owner && to > 8 || !owner && to < 3) {
        new_pieces[mov.from].kind = 4 // Hen
    }

    new_pieces
}

fn evaluate_position(pieces: Pieces) -> i32 {
    let mut result = 0;
  
    let mut board = [0u8; 12];
    for piece in pieces {
        result += (if piece.owner {-1} else {1}) * PIECE_VALUE[piece.kind as usize];
        if piece.position >= 0 {
            board[piece.position as usize] = if piece.owner { 2 } else { 1 };
        }
    }

    for piece in pieces {
        if piece.position >= 0 {
            let owner = if piece.owner {2} else {1};
            let dscore = if piece.owner {-1} else {1};
            let x = piece.position % 3;
            let y = piece.position / 3;
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

fn alphabeta(depth: u8, turn: bool, mut alpha: i32, mut beta: i32, pieces: Pieces) -> i32 {
    if depth == 0 {
        return evaluate_position(pieces)
    } else if pieces[1].position == -1 {
        return -100000-(depth as i32)
    } else if pieces[5].position == -1 {
        return 100000+(depth as i32)
    } else if turn && pieces[5].position > 8 {
        return -100000-(depth as i32)
    } else if !turn && pieces[1].position < 3 {
        return 100000+(depth as i32)
    }
    
    for mov in possible_moves(&pieces, turn) {
        let new_pieces = play_move(&pieces, mov);
        if !turn {
            let score = alphabeta(depth - 1, true, alpha, beta, new_pieces);
            if score > alpha {
                alpha = score;
            }
        } else {
            let score = alphabeta(depth - 1, false, alpha, beta, new_pieces);
            if score < beta {
                beta = score;
            }
        }
        if alpha >= beta {
            break
        }
    }
    if turn {beta} else {alpha}
}




#[tauri::command(async)]
pub fn shogi_ai(pieces: Pieces, played: Vec<Pieces>, depth: u8, turn: bool) -> Move {
    let mut alpha = i32::MIN;
    let mut beta = i32::MAX;

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
            let score = alphabeta(depth - 1, true, alpha, beta, new_pieces);
            if score > alpha {
                alpha = score;
                best_move = Some(mov);
            }
        } else {
            let score = alphabeta(depth - 1, false, alpha, beta, new_pieces);
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
            let score = alphabeta(depth - 1, true, alpha, beta, new_pieces);
            if score > alpha {
                alpha = score;
                best_move = Some(mov);
            }
        } else {
            let score = alphabeta(depth - 1, false, alpha, beta, new_pieces);
            if score < beta {
                beta = score;
                best_move = Some(mov);
            }
        }
    }
    best_move.unwrap()
}
