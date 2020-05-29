use ggez::event::{EventHandler, MouseButton};
use ggez::input;
use ggez::mint;
use ggez::nalgebra as na;
use ggez::{graphics, Context, GameResult};

use crate::board::*;

const RED: graphics::Color = graphics::Color::new(1.0, 0.0, 0.0, 1.0);
const GREEN: graphics::Color = graphics::Color::new(0.0, 1.0, 0.0, 1.0);
const BLUE: graphics::Color = graphics::Color::new(0.0, 0.0, 1.0, 1.0);
const WHITE: graphics::Color = graphics::Color::new(1.0, 1.0, 1.0, 1.0);
const LIGHT_GREY: graphics::Color = graphics::Color::new(0.5, 0.5, 0.5, 1.0);
const DARK_GREY: graphics::Color = graphics::Color::new(0.25, 0.25, 0.25, 1.0);
const BLACK: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 1.0);
const TRANSPARENT: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 0.0);

#[derive(Debug, Clone)]
pub struct Grid {
    square_size: f32,
    offset: mint::Point2<f32>,
    dragging: Option<BoardCoord>,
    drop_locations: Option<Vec<BoardCoord>>,
    board: BoardState,
}

impl Grid {
    fn to_mesh(&self, ctx: &mut Context) -> graphics::Mesh {
        let mut mesh = graphics::MeshBuilder::new();
        let fill: graphics::DrawMode = graphics::DrawMode::fill();
        let stroke_width = 10.0;
        let stroke: graphics::DrawMode = graphics::DrawMode::stroke(stroke_width);
        // Checkerboard (white square must appear on rightmost tile)
        for i in 0..8 {
            for j in 0..8 {
                let rect = graphics::Rect::new(
                    j as f32 * self.square_size,
                    i as f32 * self.square_size,
                    self.square_size,
                    self.square_size,
                );
                if (i + j) % 2 == 1 {
                    mesh.rectangle(fill, rect, LIGHT_GREY);
                } else {
                    mesh.rectangle(fill, rect, DARK_GREY);
                }
            }
        }

        // Mouse Highlight Stuff
        for i in 0..8 {
            for j in 0..8 {
                let x = j;
                let y = 7 - i;
                let rect = graphics::Rect::new(
                    j as f32 * self.square_size + stroke_width / 2.0,
                    i as f32 * self.square_size + stroke_width / 2.0,
                    self.square_size - stroke_width,
                    self.square_size - stroke_width,
                );
                let coord = BoardCoord::new((x, y)).unwrap();
                let mouse = input::mouse::position(ctx);
                let mouse = self.to_grid_coord(mouse).ok();
                // Color the dragged square green
                if Some(coord) == self.dragging {
                    mesh.rectangle(stroke, rect, GREEN);
                // Color the currently highlighted square red
                } else if Some(coord) == mouse {
                    mesh.rectangle(stroke, rect, RED);
                // Color the potential drop locations blue
                } else if contains(&self.drop_locations, &coord) {
                    mesh.rectangle(stroke, rect, BLUE);
                }
            }
        }
        mesh.build(ctx).unwrap()
    }

    fn update(&mut self, last_mouse_down_pos: Option<mint::Point2<f32>>) {
        self.dragging = match last_mouse_down_pos {
            None => None,
            Some(pos) => match self.to_grid_coord(pos) {
                Err(_) => None,
                Ok(coord) => Some(coord),
            },
        };
        self.drop_locations = self.dragging.map(|coord| self.board.get_move_list(coord));
    }

    fn draw(&self, ctx: &mut Context, font: graphics::Font) -> GameResult<()> {
        let mesh = self.to_mesh(ctx);
        graphics::draw(ctx, &mesh, (self.offset,))?;

        for i in 0..8 {
            for j in 0..8 {
                let x = j;
                let y = 7 - i;
                let coord = BoardCoord::new((x, y)).unwrap();
                let tile = self.board.get(coord);
                let mut text = graphics::Text::new(tile.as_str());
                let text = text.set_font(font, graphics::Scale::uniform(50.0));
                let location = self.to_screen_coord(BoardCoord(x, y))
                    + na::Vector2::new(self.square_size * 0.42, self.square_size * 0.25);
                let color = match tile.0 {
                    None => graphics::Color::new(0.0, 0.0, 0.0, 0.0),
                    Some(piece) => match piece.color {
                        Color::Black => BLACK,
                        Color::White => WHITE,
                    },
                };
                graphics::draw(ctx, text, (location, color))?;
            }
        }
        let text = match self.board.current_player {
            Color::Black => "Black to move",
            Color::White => "White to move",
        };
        let mut text = graphics::Text::new(text);
        let text = text.set_font(font, graphics::Scale::uniform(40.0));
        let location = self.to_screen_coord(BoardCoord(7, 7)) + na::Vector2::new(100.0, 50.0);
        graphics::draw(ctx, text, (location, RED))?;

        Ok(())
    }

    /// Returns a tuple of where the given screen space coordinates would end up
    /// on the grid. This function returns Err if the point would be off the grid.
    fn to_grid_coord(&self, screen_coords: mint::Point2<f32>) -> Result<BoardCoord, &'static str> {
        let offset_coords = minus(screen_coords, self.offset);
        let grid_x = (offset_coords.x / self.square_size).floor() as i8;
        let grid_y = (offset_coords.y / self.square_size).floor() as i8;
        BoardCoord::new((grid_x, 7 - grid_y))
    }

    /// Returns the upper left corner of the square located at `board_coords`
    fn to_screen_coord(&self, board_coords: BoardCoord) -> na::Point2<f32> {
        na::Point2::new(
            board_coords.0 as f32 * self.square_size,
            (7 - board_coords.1) as f32 * self.square_size,
        )
    }

    /// Move a piece at the location of the last mouse down press to where the
    /// mouse currently is.
    fn move_piece(&mut self, last_mouse_down_pos: mint::Point2<f32>) {
        let drop_loc = self.to_grid_coord(last_mouse_down_pos);
        if drop_loc.is_err() {
            return;
        }
        let drop_loc = drop_loc.unwrap();
        if contains(&self.drop_locations, &drop_loc) {
            self.board.take_turn(self.dragging.unwrap(), drop_loc);
        }
    }
}

/// Subtract two `mint::Point2`s from each other in the obvious way.
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

pub struct GameState {
    grid: Grid,
    font: graphics::Font,
    last_mouse_down_pos: Option<mint::Point2<f32>>, // Some(Point2<f32>) if mouse is pressed, else None
    last_mouse_up_pos: Option<mint::Point2<f32>>,
}

impl GameState {
    pub fn new(ctx: &mut Context) -> GameState {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. BK .. .. .. ..",
            ".. .. .. .. BR .. .. ..",
            ".. .. BR WK .. .. .. ..",
            ".. .. .. .. BR .. .. ..",
        ];
        let board = Board::from_string_vec(board);
        // let board = Board::default();
        let board = BoardState::new(board);

        let grid = Grid {
            square_size: 70.0,
            offset: mint::Point2 { x: 10.0, y: 10.0 },
            dragging: None,
            drop_locations: None,
            board: board,
        };

        GameState {
            grid: grid,
            font: graphics::Font::new(ctx, std::path::Path::new("\\freeserif.ttf")).unwrap(),
            last_mouse_down_pos: None,
            last_mouse_up_pos: None,
        }
    }
}

impl EventHandler for GameState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        self.grid.update(self.last_mouse_down_pos);
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
        self.grid.draw(ctx, self.font)?;
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
