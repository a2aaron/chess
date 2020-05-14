mod board;

use board::*;
use std::io;

pub fn read_string_from_stdin(message: Option<String>) -> String {
    if let Some(x) = message {
        println!("{}", x);
    }
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input = input.trim().to_string(); // Remove trailing newline
    println!("asdf: {}", input);
    input
}

fn main() {
    let mut board = Board::default();
    loop {
        println!("{}", board);
        let a: i8 = read_string_from_stdin(None).parse().unwrap();
        let b: i8 = read_string_from_stdin(None).parse().unwrap();
        println!("Chosen piece: {:?}", board.get((a, b)));
        let c: i8 = read_string_from_stdin(None).parse().unwrap();
        let d: i8 = read_string_from_stdin(None).parse().unwrap();
        println!("Goal place: {:?}", board.get((c, d)));
        let test: i8 = "1".to_string().parse().unwrap();
        println!("{:?}", board.move_piece(Color::White, (a, b), (c, d)));
    }
}
