mod board;

use ggez::event::{self, EventHandler, MouseButton};
use ggez::input;
use ggez::mint;
use ggez::nalgebra as na;
use ggez::{graphics, Context, ContextBuilder, GameResult};

use board::*;
use std::io;

pub fn read_string_from_stdin(message: Option<String>) -> String {
    if let Some(x) = message {
        println!("{}", x);
    }
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input = input.trim().to_string(); // Remove trailing newline
    input
}

fn main() {
    // Make a Context.
    let (mut ctx, mut event_loop) = ContextBuilder::new("chess", "a2aaron")
        .build()
        .expect("could not create ggez context!");

    // #[rustfmt::skip]
    // let setup = vec![
    //     ".. .. .. .. .. .. .. ..",
    //     ".. .. .. WP .. .. BP ..",
    //     ".. .. .. .. .. .. .. ..",
    //     ".. .. .. .. .. .. .. ..",
    //     ".. BP .. WQ .. .. .. ..",
    //     ".. .. .. .. .. .. .. ..",
    //     ".. BN .. .. .. .. .. ..",
    //     ".. .. .. .. .. .. WQ ..",
    // ];
    // let board = Board::from_string_vec(setup);
    let board = Board::default();
    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.
    let grid = Grid {
        square_size: 70.0,
        color: graphics::Color::new(1.0, 0.0, 0.0, 1.0),
        offset: mint::Point2 { x: 10.0, y: 10.0 },
        dragging: None,
        drop_locations: None,
        board: board,
    };

    let mut game_state = GameState {
        mesh: grid.to_mesh(&mut ctx),
        grid: grid,
        last_mouse_down_pos: None,
        last_mouse_up_pos: None,
    };

    // Run!
    match event::run(&mut ctx, &mut event_loop, &mut game_state) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
}

#[derive(Debug, Clone)]
struct Grid {
    square_size: f32,
    color: graphics::Color,
    offset: mint::Point2<f32>,
    dragging: Option<(i8, i8)>,
    drop_locations: Option<Vec<(i8, i8)>>,
    board: Board,
}

impl Grid {
    fn to_mesh(&self, ctx: &mut Context) -> graphics::Mesh {
        let mut mesh = graphics::MeshBuilder::new();
        let draw_mode = graphics::DrawMode::stroke(1.0);
        let green = graphics::Color::new(0.0, 1.0, 0.0, 1.0);
        let blue = graphics::Color::new(0.0, 0.0, 1.0, 1.0);
        for i in 0..8 {
            for j in 0..8 {
                let rect = graphics::Rect::new(
                    i as f32 * self.square_size,
                    j as f32 * self.square_size,
                    self.square_size,
                    self.square_size,
                );

                let mouse = input::mouse::position(ctx);
                if (i, j) == self.to_grid_coord(mouse) {
                    mesh.rectangle(graphics::DrawMode::fill(), rect, self.color);
                } else if Some((i, j)) == self.dragging {
                    mesh.rectangle(graphics::DrawMode::fill(), rect, green);
                } else if contains(&self.drop_locations, &(i, j)) {
                    mesh.rectangle(graphics::DrawMode::fill(), rect, blue);
                } else {
                    mesh.rectangle(draw_mode, rect, self.color);
                }
            }
        }
        mesh.build(ctx).unwrap()
    }

    fn update(&mut self, last_mouse_down_pos: Option<mint::Point2<f32>>) {
        self.dragging = last_mouse_down_pos.map(|pos| self.to_grid_coord(pos));
        self.drop_locations = self
            .dragging
            .map(|coord| get_move_list(&self.board, coord).0);
    }

    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        let mesh = self.to_mesh(ctx);
        graphics::draw(ctx, &mesh, (self.offset,))
    }

    fn to_grid_coord(&self, screen_coords: mint::Point2<f32>) -> (i8, i8) {
        let offset_coords = minus(screen_coords, self.offset);
        let grid_x = (offset_coords.x / self.square_size).floor() as i8;
        let grid_y = (offset_coords.y / self.square_size).floor() as i8;
        (grid_x, grid_y)
    }

    fn move_piece(&mut self, last_mouse_down_pos: mint::Point2<f32>) {
        let drop_loc = self.to_grid_coord(last_mouse_down_pos);
        if contains(&self.drop_locations, &drop_loc) {
            self.board.move_piece(self.dragging.unwrap(), drop_loc);
        }
    }
}

fn minus(a: mint::Point2<f32>, b: mint::Point2<f32>) -> mint::Point2<f32> {
    mint::Point2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn contains<T, V>(vec: &Option<V>, thing: &T) -> bool
where
    T: Eq,
    V: AsRef<[T]>,
{
    vec.as_ref()
        .map(|vec| vec.as_ref().contains(thing))
        .unwrap_or(false)
}

struct GameState {
    grid: Grid,
    mesh: graphics::Mesh,
    last_mouse_down_pos: Option<mint::Point2<f32>>, // Some(Point2<f32>) if mouse is pressed, else None
    last_mouse_up_pos: Option<mint::Point2<f32>>,
}

impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        /*let board_state = self.board_state;
        println!("{}", board_state.board);
        println!("{} to move. Select a piece:", board_state.current_player);
        let a: i8 = read_string_from_stdin(None).parse().unwrap();
        let b: i8 = read_string_from_stdin(None).parse().unwrap();
        println!("Chosen piece: {:?}", board_state.board.get((a, b)));
        println!("Select an end location");
        let move_list = get_move_list(&board_state.board, (a, b));
        for y in (0..8).rev() {
            for x in 0..8 {
                if move_list.0.contains(&(x, y)) {
                    print!("## ");
                } else {
                    print!("{} ", board_state.board.get((x, y)));
                }
            }
            println!("");
        }
        let c: i8 = read_string_from_stdin(None).parse().unwrap();
        let d: i8 = read_string_from_stdin(None).parse().unwrap();
        println!("Goal place: {:?}", board_state.board.get((c, d)));
        let result = board_state.take_turn((a, b), (c, d));
        println!("{:?}", result);*/
        self.grid.update(self.last_mouse_down_pos);
        self.mesh = self.grid.to_mesh(ctx);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);
        let circle = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            na::Point2::new(0.0, 0.0),
            10.0,
            2.0,
            graphics::WHITE,
        )?;
        let pos = input::mouse::position(ctx);
        let (mousex, mousey) = (pos.x, pos.y);
        self.grid.draw(ctx)?;
        graphics::draw(ctx, &circle, (na::Point2::new(mousex, mousey),))?;
        graphics::present(ctx)
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        x: f32,
        y: f32,
    ) {
        self.last_mouse_down_pos = Some(mint::Point2 { x, y });
        self.last_mouse_up_pos = None;
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {
        self.last_mouse_down_pos = None;
        self.last_mouse_up_pos = Some(mint::Point2 { x, y });
        self.grid.move_piece(mint::Point2 { x, y });
    }
}
