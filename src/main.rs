mod board;

use board::*;
use std::fmt;

fn main() -> Result<(), &'static str> {
    let mut board = Board::default();
    println!("{}", board);

    println!("moved a white pawn!");
    board
        .move_piece(Color::White, (0, 1), (0, 2))
        .expect("oh god oh fuck it broke");
    println!("{}", board);

    println!("captured a white pawn with a black pawn!");
    board.move_piece(Color::Black, (3, 6), (3, 5))?;
    board.move_piece(Color::Black, (3, 5), (3, 4))?;
    board.move_piece(Color::Black, (3, 4), (3, 3))?;
    board.move_piece(Color::Black, (3, 3), (3, 2))?;
    board.move_piece(Color::Black, (3, 2), (4, 1))?;
    println!("{}", board);

    return Ok(());
}
