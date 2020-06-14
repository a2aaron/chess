use crate::board::*;

use ggez::event::{EventHandler, MouseButton};
use ggez::graphics::{self, DrawParam, Rect};
use ggez::input;
use ggez::mint;
use ggez::nalgebra as na;
use ggez::{Context, GameResult};

pub const SCREEN_WIDTH: f32 = 800.0;
pub const SCREEN_HEIGHT: f32 = 600.0;

const RED: graphics::Color = graphics::Color::new(1.0, 0.0, 0.0, 1.0);
const GREEN: graphics::Color = graphics::Color::new(0.0, 1.0, 0.0, 1.0);
const BLUE: graphics::Color = graphics::Color::new(0.0, 0.0, 1.0, 1.0);
const WHITE: graphics::Color = graphics::Color::new(1.0, 1.0, 1.0, 1.0);
const LIGHT_GREY: graphics::Color = graphics::Color::new(0.5, 0.5, 0.5, 1.0);
const DARK_GREY: graphics::Color = graphics::Color::new(0.25, 0.25, 0.25, 1.0);
const BLACK: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 1.0);
const TRANSPARENT: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 0.0);

#[derive(Debug)]
pub struct Game {
    screen: ScreenState,
    title_screen: TitleScreen,
    grid: Grid,
    font: graphics::Font,
    last_mouse_down_pos: Option<mint::Point2<f32>>, // Some(Point2<f32>) if mouse is pressed, else None
    last_mouse_up_pos: Option<mint::Point2<f32>>,
    last_screen_state: ScreenState, // used to detect state transitions
}

impl Game {
    pub fn new(ctx: &mut Context) -> Game {
        Game {
            screen: ScreenState::TitleScreen,
            title_screen: TitleScreen::new(),
            grid: Grid::new(ctx),
            font: graphics::Font::new(ctx, std::path::Path::new("\\freeserif.ttf")).unwrap(),
            last_mouse_down_pos: None,
            last_mouse_up_pos: None,
            last_screen_state: ScreenState::TitleScreen,
        }
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // ScreenState transitions
        // todo: this seems silly
        match (self.last_screen_state, self.screen) {
            (ScreenState::TitleScreen, ScreenState::InGame) => self.grid.new_game(),
            _ => (),
        }

        match self.screen {
            ScreenState::TitleScreen => self.title_screen.upd8(ctx),
            ScreenState::InGame => self.grid.upd8(ctx),
            ScreenState::Quit => ggez::event::quit(ctx),
        }

        self.last_screen_state = self.screen;
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

        match self.screen {
            ScreenState::TitleScreen => self.title_screen.draw(ctx, self.font)?,
            ScreenState::InGame => self.grid.draw(ctx, self.font)?,
            ScreenState::Quit => (),
        }
        graphics::draw(ctx, &circle, (na::Point2::new(mousex, mousey),))?;

        // FPS counter
        let text = format!("{}", ggez::timer::fps(ctx));
        let location = na::Point2::new(100.0, 500.0);
        draw_text(ctx, text, self.font, 20.0, (location, RED))?;

        graphics::present(ctx)
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        x: f32,
        y: f32,
    ) {
        let pos = mint::Point2 { x, y };
        self.last_mouse_down_pos = Some(pos);
        self.last_mouse_up_pos = None;
        match self.screen {
            ScreenState::TitleScreen => (),
            ScreenState::InGame => self.grid.mouse_down_upd8(pos),
            ScreenState::Quit => (),
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {
        self.last_mouse_down_pos = None;
        self.last_mouse_up_pos = Some(mint::Point2 { x, y });

        match self.screen {
            ScreenState::TitleScreen => self
                .title_screen
                .mouse_up_upd8(mint::Point2 { x, y }, &mut self.screen),
            ScreenState::InGame => self
                .grid
                .mouse_up_upd8(mint::Point2 { x, y }, &mut self.screen),
            ScreenState::Quit => (),
        }
    }
}

#[derive(Debug)]
pub struct Grid {
    square_size: f32,
    offset: na::Vector2<f32>,
    dragging: Option<BoardCoord>,
    drop_locations: Vec<BoardCoord>,
    board: BoardState,
    background_mesh: graphics::Mesh,
    restart: Button,
    main_menu: Button,
    queen_button: Button, // todo: this is probably dumb, use a vector later on
    rook_button: Button,
    bishop_button: Button,
    knight_button: Button,
}
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum UIState {
    Normal,
    Promote(BoardCoord),
    GameOver,
}

impl Grid {
    fn new(ctx: &mut Context) -> Grid {
        Grid {
            square_size: 70.0,
            offset: na::Vector2::new(10.0, 10.0),
            dragging: None,
            drop_locations: vec![],
            board: BoardState::new(Board::default()),
            background_mesh: Grid::background_mesh(ctx, 70.0),
            restart: Button::new(
                Rect::new(SCREEN_WIDTH * 0.75, SCREEN_HEIGHT / 2.0 - 40.0, 100.0, 35.0),
                "Restart Game",
            ),
            main_menu: Button::new(
                Rect::new(SCREEN_WIDTH * 0.75, SCREEN_HEIGHT / 2.0 + 40.0, 100.0, 35.0),
                "Main Menu",
            ),
            queen_button: Button::new(
                Rect::new(SCREEN_WIDTH * 0.75, SCREEN_HEIGHT - 40.0, 40.0, 35.0),
                QUEEN_STR,
            ),
            rook_button: Button::new(
                Rect::new(SCREEN_WIDTH * 0.75 + 40.0, SCREEN_HEIGHT - 40.0, 40.0, 35.0),
                ROOK_STR,
            ),
            bishop_button: Button::new(
                Rect::new(
                    SCREEN_WIDTH * 0.75 + 40.0 * 2.0,
                    SCREEN_HEIGHT - 40.0,
                    40.0,
                    35.0,
                ),
                BISHOP_STR,
            ),
            knight_button: Button::new(
                Rect::new(
                    SCREEN_WIDTH * 0.75 + 40.0 * 3.0,
                    SCREEN_HEIGHT - 40.0,
                    40.0,
                    35.0,
                ),
                KNIGHT_STR,
            ),
        }
    }

    fn new_game(&mut self) {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            "WP .. .. .. .. BK .. ..",
            ".. WP .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "BP .. .. .. .. .. .. ..",
            ".. BP .. .. .. .. .. WK",
            ".. .. .. .. .. .. .. ..",
        ];
        let board = Board::from_string_vec(board);
        // let board = Board::default();
        self.board = BoardState::new(board);
        self.dragging = None;
        self.drop_locations = vec![];
    }

    fn upd8(&mut self, ctx: &mut Context) {
        use UIState::*;
        match self.ui_state() {
            Normal => (),
            GameOver => {
                self.main_menu.upd8(ctx);
                self.restart.upd8(ctx);
            }
            Promote(_) => {
                self.queen_button.upd8(ctx);
                self.rook_button.upd8(ctx);
                self.bishop_button.upd8(ctx);
                self.knight_button.upd8(ctx);
            }
        }
    }

    fn mouse_down_upd8(&mut self, mouse_pos: mint::Point2<f32>) {
        let coord = self.to_grid_coord(mouse_pos);
        match coord {
            Err(_) => {
                self.dragging = None;
                self.drop_locations = vec![];
            }
            Ok(coord) => {
                self.dragging = Some(coord);
                self.drop_locations = self.board.get_move_list(coord);
            }
        };
    }

    fn mouse_up_upd8(&mut self, mouse_pos: mint::Point2<f32>, screen_state: &mut ScreenState) {
        use UIState::*;
        match self.ui_state() {
            Normal => self.move_piece(mouse_pos),
            GameOver => {
                if self.restart.pressed(mouse_pos) {
                    self.new_game();
                }

                if self.main_menu.pressed(mouse_pos) {
                    *screen_state = ScreenState::TitleScreen;
                }
            }
            Promote(coord) => {
                if self.queen_button.pressed(mouse_pos) {
                    self.board
                        .promote(coord, PieceType::Queen)
                        .expect("Expected promotion to work");
                }
                if self.rook_button.pressed(mouse_pos) {
                    self.board
                        .promote(coord, PieceType::Rook)
                        .expect("Expected promotion to work");
                }
                if self.bishop_button.pressed(mouse_pos) {
                    self.board
                        .promote(coord, PieceType::Bishop)
                        .expect("Expected promotion to work");
                }
                if self.knight_button.pressed(mouse_pos) {
                    self.board
                        .promote(coord, PieceType::Knight)
                        .expect("Expected promotion to work");
                }
            }
        }
        self.dragging = None;
        self.drop_locations = vec![];
    }

    fn draw(&self, ctx: &mut Context, font: graphics::Font) -> GameResult<()> {
        graphics::draw(ctx, &self.background_mesh, (na::Point2::from(self.offset),))?;
        if self.ui_state() == UIState::Normal {
            self.draw_highlights(ctx)?;
        }

        self.draw_pieces(ctx, font)?;

        // Draw UI buttons, if applicable
        use UIState::*;
        match self.ui_state() {
            Normal => (),
            GameOver => {
                self.draw_game_over(ctx, font)?;
            }
            Promote(_) => {
                self.queen_button.draw(ctx, font)?;
                self.rook_button.draw(ctx, font)?;
                self.bishop_button.draw(ctx, font)?;
                self.knight_button.draw(ctx, font)?;
            }
        }

        // TODO: It is probably better to store this as a text mesh? Maybe pregenerate
        // all the possible texts I want to draw?
        let text = match self.board.current_player {
            Color::Black => "Black to move",
            Color::White => "White to move",
        };
        let location = self.to_screen_coord(BoardCoord(7, 7)) + na::Vector2::new(100.0, 50.0);
        draw_text(ctx, text, font, 40.0, (location, RED))?;

        if let Some(coord) = self.board.need_promote() {
            let text = format!("Pawn at {:?} needs promotion!", coord);
            let location = self.to_screen_coord(BoardCoord(7, 7)) + na::Vector2::new(100.0, 400.0);
            draw_text(ctx, text, font, 20.0, (location, RED))?;
        }

        Ok(())
    }

    /// Move a piece at the location of the last mouse down press to where the
    /// mouse currently is.
    fn move_piece(&mut self, mouse_pos: mint::Point2<f32>) {
        let drop_loc = self.to_grid_coord(mouse_pos);
        if drop_loc.is_err() {
            return;
        }
        let drop_loc = drop_loc.unwrap();
        println!(
            "{:?}",
            self.board.take_turn(self.dragging.unwrap(), drop_loc)
        );
    }

    fn draw_pieces(&self, ctx: &mut Context, font: graphics::Font) -> GameResult<()> {
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
                    None => TRANSPARENT,
                    Some(piece) => match piece.color {
                        Color::Black => BLACK,
                        Color::White => WHITE,
                    },
                };
                graphics::draw(ctx, text, (location, color))?;
            }
        }
        Ok(())
    }

    fn draw_highlights(&self, ctx: &mut Context) -> GameResult<()> {
        let fill: graphics::DrawMode = graphics::DrawMode::fill();

        let mut mesh = graphics::MeshBuilder::new();
        let solid_rect = Rect::new(0.0, 0.0, self.square_size, self.square_size);
        mesh.rectangle(fill, solid_rect, RED);
        let solid_rect = mesh.build(ctx).unwrap();

        // TODO: this is an awful idea, instead expose a field similar to self.board.checkmate
        let king_coord = self.board.board.get_king(self.board.current_player);

        // If king in check, draw it in red.
        if let Some(coord) = king_coord {
            if self.board.checkmate != CheckmateState::Normal {
                let offset = self.to_screen_coord(coord) + self.offset;
                graphics::draw(ctx, &solid_rect, (offset,))?;
            }
        }

        let stroke_width = 10.0;
        let stroke: graphics::DrawMode = graphics::DrawMode::stroke(stroke_width);

        let mut mesh = graphics::MeshBuilder::new();
        let hollow_rect = Rect::new(
            stroke_width / 2.0,
            stroke_width / 2.0,
            self.square_size - stroke_width,
            self.square_size - stroke_width,
        );
        // don't actually care about the color here
        mesh.rectangle(stroke, hollow_rect, WHITE);
        let hollow_rect = mesh.build(ctx).unwrap();

        // Mouse Highlight Stuff
        let mouse = input::mouse::position(ctx);
        let mouse = self.to_grid_coord(mouse).ok();
        for coord in &self.drop_locations {
            let offset: na::Point2<f32> = self.to_screen_coord(*coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, BLUE))?;
        }

        // Color the currently highlighted square red if it is the player's piece
        if let Some(coord) = mouse {
            let same_color = self.board.get(coord).is_color(self.board.current_player);
            let dragging = self.dragging.is_some();
            let color = if same_color || dragging {
                RED
            } else {
                TRANSPARENT
            };
            let offset: na::Point2<f32> = self.to_screen_coord(coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, color))?;
        }

        // Color the dragged square green
        if let Some(coord) = self.dragging {
            let offset: na::Point2<f32> = self.to_screen_coord(coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, GREEN))?;
        }

        Ok(())
    }

    fn draw_game_over(&self, ctx: &mut Context, font: graphics::Font) -> GameResult<()> {
        let player_str = self.board.current_player.as_str();

        let text = match self.board.checkmate {
            CheckmateState::Stalemate => "The game has ended in a stalemate!".to_owned(),
            CheckmateState::Checkmate => {
                ["The game has ended!\n", player_str, " is in checkmate!"].concat()
            }
            _ => unreachable!(),
        };
        let location = self.to_screen_coord(BoardCoord(7, 7)) + na::Vector2::new(100.0, 200.0);
        draw_text(ctx, text, font, 20.0, (location, RED))?;

        // let fill: graphics::DrawMode = graphics::DrawMode::fill();
        // let mut mesh = graphics::MeshBuilder::new();
        // let solid_rect = Rect::new(0.0, 0.0, self.square_size, self.square_size);
        // mesh.rectangle(fill, solid_rect, WHITE);
        // let solid_rect = mesh.build(ctx).unwrap();

        self.restart.draw(ctx, font)?;
        self.main_menu.draw(ctx, font)?;

        Ok(())
    }

    fn background_mesh(ctx: &mut Context, square_size: f32) -> graphics::Mesh {
        let mut mesh = graphics::MeshBuilder::new();
        let fill: graphics::DrawMode = graphics::DrawMode::fill();
        for i in 0..8 {
            for j in 0..8 {
                let rect = Rect::new(
                    j as f32 * square_size,
                    i as f32 * square_size,
                    square_size,
                    square_size,
                );
                if (i + j) % 2 == 1 {
                    mesh.rectangle(fill, rect, LIGHT_GREY);
                } else {
                    mesh.rectangle(fill, rect, DARK_GREY);
                }
            }
        }

        mesh.build(ctx).unwrap()
    }

    fn ui_state(&self) -> UIState {
        match (self.board.game_over(), self.board.need_promote()) {
            (false, None) => UIState::Normal,
            (false, Some(coord)) => UIState::Promote(coord),
            (true, _) => UIState::GameOver,
        }
    }

    /// Returns a tuple of where the given screen space coordinates would end up
    /// on the grid. This function returns Err if the point would be off the grid.
    fn to_grid_coord(&self, screen_coords: mint::Point2<f32>) -> Result<BoardCoord, &'static str> {
        let offset_coords: na::Point2<f32> = na::Point2::from(screen_coords) - self.offset;
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
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum ButtonState {
    Idle,
    Hover,
    Pressed,
}

#[derive(Debug)]
pub struct Button {
    hitbox: Rect,
    text: String,
    state: ButtonState,
}

impl Button {
    fn new(hitbox: Rect, text: &str) -> Button {
        Button {
            hitbox: hitbox,
            text: text.to_owned(),
            state: ButtonState::Idle,
        }
    }

    fn pressed(&self, mouse_pos: mint::Point2<f32>) -> bool {
        self.state == ButtonState::Pressed && self.hitbox.contains(mouse_pos)
    }

    fn upd8(&mut self, ctx: &mut Context) {
        let curr_pos = input::mouse::position(ctx);
        let mouse_pressed = input::mouse::button_pressed(ctx, MouseButton::Left);
        let over_button = self.hitbox.contains(curr_pos);
        use ButtonState::*;
        self.state = match (over_button, mouse_pressed) {
            (false, _) => Idle,
            (true, false) => Hover,
            (true, true) => Pressed,
        };
    }

    fn draw(&self, ctx: &mut Context, font: graphics::Font) -> GameResult<()> {
        use ButtonState::*;

        let fill: graphics::DrawMode = graphics::DrawMode::fill();
        let stroke_width = 3.0;
        let stroke: graphics::DrawMode = graphics::DrawMode::stroke(stroke_width);

        let outer_color = WHITE;
        let inner_color = match self.state {
            Idle => graphics::Color::from_rgb_u32(0x13ff00),
            Hover => graphics::Color::from_rgb_u32(0x0ebf00),
            Pressed => graphics::Color::from_rgb_u32(0x0c9f00),
        };

        let dims = get_dims(self.hitbox);
        let dest = self.hitbox.point();

        // Button BG
        let mut mesh = graphics::MeshBuilder::new();
        mesh.rectangle(fill, dims, inner_color);

        // Button Border
        let bounds = Rect::new(
            stroke_width / 2.0,
            stroke_width / 2.0,
            self.hitbox.w - stroke_width,
            self.hitbox.h - stroke_width,
        );
        mesh.rectangle(stroke, bounds, outer_color);
        let button = mesh.build(ctx).unwrap();

        graphics::draw(ctx, &button, (dest,))?;

        let location = get_center(self.hitbox);
        draw_text_centered(ctx, self.text.as_str(), font, 20.0, (location, BLACK))?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct TitleScreen {
    start_game: Button,
    quit_game: Button,
}

impl TitleScreen {
    fn new() -> TitleScreen {
        TitleScreen {
            start_game: Button::new(
                center(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT * 0.50, 300.0, 35.0),
                "Start Game",
            ),
            quit_game: Button::new(
                center(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT * 0.75, 300.0, 35.0),
                "Quit Game",
            ),
        }
    }

    fn upd8(&mut self, ctx: &mut Context) {
        self.start_game.upd8(ctx);
        self.quit_game.upd8(ctx);
    }

    fn mouse_up_upd8(&mut self, mouse_pos: mint::Point2<f32>, screen_state: &mut ScreenState) {
        if self.start_game.pressed(mouse_pos) {
            *screen_state = ScreenState::InGame;
        }

        if self.quit_game.pressed(mouse_pos) {
            *screen_state = ScreenState::Quit;
        }
    }

    fn draw(&self, ctx: &mut Context, font: graphics::Font) -> GameResult<()> {
        self.start_game.draw(ctx, font)?;
        self.quit_game.draw(ctx, font)?;

        let scale = 60.0;
        let text = "CHESS";
        let location = mint::Point2 {
            x: SCREEN_WIDTH / 2.0,
            y: SCREEN_HEIGHT * 0.25,
        };
        draw_text_centered(ctx, text, font, scale, (location, RED))?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ScreenState {
    TitleScreen,
    InGame,
    Quit,
}

// Returns a rect such that its center is located (x, y). Assumes that the
// upper left corner of the Rect is where (x, y) is and that rectangles
// grow to the right and downwards.
fn center(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x - w / 2.0, y - h / 2.0, w, h)
}

// Returns a point located at the center of the rectangle. Assumes that the
// upper left corner of the Rect is where (x, y) is and that rectangles
// grow to the right and downwards.
fn get_center(rect: Rect) -> mint::Point2<f32> {
    mint::Point2 {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
    }
}

// Returns a rectangle located at (0, 0) with dimensions (w, h)
fn get_dims(rect: Rect) -> Rect {
    Rect::new(0.0, 0.0, rect.w, rect.h)
}

// Return a rectangle the same size as inner, centered inside of outer
fn center_inside(outer: Rect, inner: Rect) -> Rect {
    let point = get_center(outer);
    center(point.x, point.y, inner.w, inner.h)
}

// Draw some text using the font, scale, and parameters specified.
fn draw_text<T, S>(
    ctx: &mut Context,
    text: T,
    font: graphics::Font,
    scale: f32,
    params: S,
) -> GameResult<()>
where
    T: Into<graphics::TextFragment>,
    S: Into<DrawParam>,
{
    let mut text = graphics::Text::new(text);
    let text = text.set_font(font, graphics::Scale::uniform(scale));
    graphics::draw(ctx, text, params)
}

// Draw some text using the font, scale, and parameters specified.
// Note that the destination in the `DrawParams` is altered to be the center of
// the text.
fn draw_text_centered<T, S>(
    ctx: &mut Context,
    text: T,
    font: graphics::Font,
    scale: f32,
    params: S,
) -> GameResult<()>
where
    T: Into<graphics::TextFragment>,
    S: Into<DrawParam>,
{
    let params: DrawParam = params.into();

    // Make the text stuff
    let mut text = graphics::Text::new(text);
    let text = text.set_font(font, graphics::Scale::uniform(scale));

    // Find the point such that the text will be centered on `param.dest`
    let (width, height) = text.dimensions(ctx);
    let location = mint::Point2 {
        x: params.dest.x - width as f32 / 2.0,
        y: params.dest.y - height as f32 / 2.0,
    };
    let params = params.dest(location);

    graphics::draw(ctx, text, params)
}
