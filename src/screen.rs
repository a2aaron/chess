use crate::board::*;
use crate::layout::*;
use crate::rect::*;

use ggez::event::{EventHandler, MouseButton};
use ggez::graphics::{self, DrawParam, Rect, Text};
use ggez::input;
use ggez::mint;
use ggez::nalgebra as na;
use ggez::{Context, GameResult};

pub const SCREEN_WIDTH: f32 = 800.0;
pub const SCREEN_HEIGHT: f32 = 600.0;

const DEFAULT_SCALE: f32 = 20.0;
const DONTCARE: f32 = -999.0;

const RED: graphics::Color = graphics::Color::new(1.0, 0.0, 0.0, 1.0);
const GREEN: graphics::Color = graphics::Color::new(0.0, 1.0, 0.0, 1.0);
const BLUE: graphics::Color = graphics::Color::new(0.0, 0.0, 1.0, 1.0);
const WHITE: graphics::Color = graphics::Color::new(1.0, 1.0, 1.0, 1.0);
const LIGHT_GREY: graphics::Color = graphics::Color::new(0.5, 0.5, 0.5, 1.0);
const DARK_GREY: graphics::Color = graphics::Color::new(0.25, 0.25, 0.25, 1.0);
const BLACK: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 1.0);
const TRANSPARENT: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 0.0);
const TRANS_RED: graphics::Color = graphics::Color::new(1.0, 0.0, 0.0, 0.5);
const TRANS_YELLOW: graphics::Color = graphics::Color::new(1.0, 1.0, 0.0, 0.5);
const TRANS_GREEN: graphics::Color = graphics::Color::new(0.0, 1.0, 0.0, 0.5);
const TRANS_CYAN: graphics::Color = graphics::Color::new(0.0, 1.0, 1.0, 0.5);
const TRANS_BLUE: graphics::Color = graphics::Color::new(0.0, 0.0, 1.0, 0.5);
const TRANS_PURPLE: graphics::Color = graphics::Color::new(1.0, 0.0, 1.0, 0.5);
#[derive(Debug)]
pub struct Game {
    screen: ScreenState,
    title_screen: TitleScreen,
    grid: Grid,
    last_screen_state: ScreenState, // used to detect state transitions
    ext_ctx: ExtendedContext,
}

#[derive(Debug)]
pub struct ExtendedContext {
    mouse_state: MouseState,
    font: graphics::Font,
    debug_render: Vec<(Rect, graphics::Color)>, // the debug rectangles. Rendered in red on top of everything else.
}

impl ExtendedContext {
    fn new(ctx: &mut Context, font: graphics::Font) -> ExtendedContext {
        ExtendedContext {
            mouse_state: MouseState::new(ctx),
            font,
            debug_render: vec![],
        }
    }
}

#[derive(Debug)]
pub struct MouseState {
    last_down: Option<mint::Point2<f32>>, // The position of the last mouse down, if it exists
    last_up: Option<mint::Point2<f32>>,   // The position of the last mouse up, if it exists
    dragging: Option<mint::Point2<f32>>,  // Some(coord) if the mouse is pressed, else None
    pos: mint::Point2<f32>,
}

impl MouseState {
    fn new(ctx: &mut Context) -> MouseState {
        MouseState {
            last_down: None,
            last_up: None,
            dragging: None,
            pos: ggez::input::mouse::position(ctx),
        }
    }
}

impl Game {
    pub fn new(ctx: &mut Context) -> Game {
        let font = graphics::Font::new(ctx, std::path::Path::new("\\freeserif.ttf")).unwrap();
        let mut ext_ctx = ExtendedContext::new(ctx, font);

        Game {
            screen: ScreenState::TitleScreen,
            title_screen: TitleScreen::new(ctx, font),
            grid: Grid::new(ctx, &mut ext_ctx),
            ext_ctx: ext_ctx,
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
            ScreenState::InGame => self.grid.upd8(ctx, &mut self.ext_ctx),
            ScreenState::Quit => ggez::event::quit(ctx),
        }

        self.last_screen_state = self.screen;

        self.ext_ctx.mouse_state.pos = ggez::input::mouse::position(ctx);

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

        match self.screen {
            ScreenState::TitleScreen => self.title_screen.draw(ctx, self.ext_ctx.font)?,
            ScreenState::InGame => {
                self.grid
                    .draw(ctx, &self.ext_ctx.mouse_state, self.ext_ctx.font)?
            }
            ScreenState::Quit => (),
        }
        graphics::draw(ctx, &circle, (self.ext_ctx.mouse_state.pos,))?;

        // FPS counter
        let text = format!("{}", ggez::timer::fps(ctx));
        let location = na::Point2::new(100.0, 500.0);
        draw_text(ctx, text, self.ext_ctx.font, DEFAULT_SCALE, (location, RED))?;

        // Debug Rects
        for (rect, color) in &self.ext_ctx.debug_render {
            if rect.x == DONTCARE || rect.y == DONTCARE {
                println!("unset rect position! {:?}", rect);
            }
            let rect =
                graphics::Mesh::new_rectangle(ctx, graphics::DrawMode::fill(), *rect, *color)
                    .unwrap();
            graphics::draw(ctx, &rect, DrawParam::default())?;
        }
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

        // self.mouse_state.last_up = None;
        self.ext_ctx.mouse_state.last_down = Some(pos);
        self.ext_ctx.mouse_state.dragging = Some(pos);

        match self.screen {
            ScreenState::TitleScreen => (),
            ScreenState::InGame => self.grid.mouse_down_upd8(pos),
            ScreenState::Quit => (),
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {
        // self.mouse_state.last_down = None;
        self.ext_ctx.mouse_state.last_up = Some(mint::Point2 { x, y });
        self.ext_ctx.mouse_state.dragging = None;

        match self.screen {
            ScreenState::TitleScreen => self
                .title_screen
                .mouse_up_upd8(mint::Point2 { x, y }, &mut self.screen),
            ScreenState::InGame => {
                self.grid
                    .mouse_up_upd8(ctx, &self.ext_ctx.mouse_state, &mut self.screen)
            }
            ScreenState::Quit => (),
        }
    }
}

#[derive(Debug)]
pub struct Grid {
    square_size: f32,
    offset: na::Vector2<f32>,
    drop_locations: Vec<BoardCoord>,
    board: BoardState,
    background_mesh: graphics::Mesh,
    restart: Button,
    main_menu: Button,
    promote_buttons: Vec<(Button, PieceType)>,
    status: TextBox,
}
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum UIState {
    Normal,
    Promote(BoardCoord),
    GameOver,
}

impl Grid {
    fn new(ctx: &mut Context, ext_ctx: &mut ExtendedContext) -> Grid {
        use PieceType::*;
        let square_size = 70.0;
        let font = ext_ctx.font;

        let button_size = Rect::new(DONTCARE, DONTCARE, 40.0, 35.0);
        let promote_buttons = vec![
            (
                Button::fit_to_text(ctx, button_size, text(QUEEN_STR, font, 40.0)),
                Queen,
            ),
            (
                Button::fit_to_text(ctx, button_size, text(ROOK_STR, font, 40.0)),
                Rook,
            ),
            (
                Button::fit_to_text(ctx, button_size, text(BISHOP_STR, font, 40.0)),
                Bishop,
            ),
            (
                Button::fit_to_text(ctx, button_size, text(KNIGHT_STR, font, 40.0)),
                Knight,
            ),
        ];

        let mut grid = Grid {
            square_size,
            offset: na::Vector2::new(DONTCARE, DONTCARE),
            drop_locations: vec![],
            board: BoardState::new(Board::default()),
            background_mesh: Grid::background_mesh(ctx, square_size),
            restart: Button::new(
                from_dims((100.0, 35.0)),
                text("Restart Game", font, DEFAULT_SCALE),
            ),
            main_menu: Button::new(
                from_dims((100.0, 35.0)),
                text("Main Menu", font, DEFAULT_SCALE),
            ),
            status: TextBox {
                bounding_box: from_dims((110.0, 100.0)),
                text: Text::default(),
            },
            promote_buttons,
        };
        grid.relayout(ext_ctx);
        grid
    }

    fn relayout(&mut self, ext_ctx: &mut ExtendedContext) {
        let off_x = 10.0;
        let off_y = 10.0;

        let grid_bounding = Rect::new(off_x, off_y, 8.0 * self.square_size, 8.0 * self.square_size);

        // the right empty margin
        let margin = from_points(
            grid_bounding.right(),
            grid_bounding.top(),
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
        );

        let button_size = self.promote_buttons[0].0.hitbox;
        let mut layout_buttons: Vec<&mut dyn Layout> = vec![];
        for (button, _) in self.promote_buttons.iter_mut() {
            layout_buttons.push(button);
        }

        let mut grid = Rect::new(off_x, off_y, 8.0 * self.square_size, 8.0 * self.square_size);
        let mut button_stack = HStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut layout_buttons[..],
            min_dimensions: (None, None),
        };
        let mut menu_buttons = VStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut [&mut self.restart, &mut self.main_menu],
            min_dimensions: (None, None),
        };

        // Same size as the buttons, but used as padding
        let mut fake_stack = HStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut [
                &mut button_size.clone(),
                &mut button_size.clone(),
                &mut button_size.clone(),
                &mut button_size.clone(),
            ],
            min_dimensions: (None, None),
        };

        let mut padding1 = FlexBox::new(1.0);
        let mut padding2 = FlexBox::new(1.0);
        let mut sidebar_children: [&mut dyn Layout; 6] = match self.board.current_player {
            Color::White => [
                &mut fake_stack,
                &mut padding1,
                &mut self.status,
                &mut menu_buttons,
                &mut padding2,
                &mut button_stack,
            ],
            Color::Black => [
                &mut button_stack,
                &mut padding1,
                &mut self.status,
                &mut menu_buttons,
                &mut padding2,
                &mut fake_stack,
            ],
        };

        let mut sidebar = VStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut sidebar_children,
            min_dimensions: (Some(SCREEN_WIDTH - grid.right() - 10.0), Some(grid.h)),
        };

        let mut padding3 = FlexBox::new(1.0);
        let mut full_ui = HStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut [&mut grid, &mut padding3, &mut sidebar],
            min_dimensions: (Some(SCREEN_WIDTH - 20.0), None),
        };

        full_ui.layout((SCREEN_WIDTH - 20.0, SCREEN_HEIGHT - 20.0));

        full_ui.set_position_relative(mint::Vector2 { x: 10.0, y: 10.0 });

        // DEBUG
        // ext_ctx.debug_render.clear();
        // ext_ctx.debug_render.push((grid, TRANS_BLUE));
        // ext_ctx
        //     .debug_render
        //     .push((sidebar.bounding_box(), TRANS_YELLOW));
        // ext_ctx
        //     .debug_render
        //     .push((menu_buttons.bounding_box(), TRANS_RED));
        // ext_ctx.debug_render.push((padding1.bounding_box(), RED));
        // ext_ctx.debug_render.push((padding2.bounding_box(), BLUE));
        // ext_ctx.debug_render.push((padding3.bounding_box(), WHITE));
        // println!("padding3 {:?}", padding3.bounding_box());
        // ext_ctx
        //     .debug_render
        //     .push((button_stack.bounding_box(), TRANS_PURPLE));
        // ext_ctx
        //     .debug_render
        //     .push((self.status.bounding_box, TRANS_CYAN));

        // ext_ctx
        //     .debug_render
        //     .push((fake_stack.bounding_box(), TRANS_GREEN));

        // todo: maybe make this stuff a layout and dont do Weird Rectangle
        self.offset = na::Vector2::new(grid.x, grid.y);
    }

    fn new_game(&mut self) {
        let board = vec![
            ".. .. .. .. .. .. .. ..",
            "WP .. .. .. .. BK .. ..",
            ".. WP .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            ".. .. .. .. .. .. .. ..",
            "BP .. .. .. .. .. .. ..",
            ".. BR .. .. .. .. .. WK",
            ".. .. BR .. .. .. .. ..",
        ];
        let board = Board::from_string_vec(board);
        // let board = Board::default();
        self.board = BoardState::new(board);
        self.drop_locations = vec![];
    }

    fn upd8(&mut self, ctx: &mut Context, ext_ctx: &mut ExtendedContext) {
        // Update status message
        let player_str = self.board.current_player.as_str();

        let status_text = match self.board.checkmate {
            CheckmateState::Stalemate => "The game has ended in a stalemate!".to_owned(),
            CheckmateState::Checkmate => {
                ["The game has ended!\n", player_str, " is in checkmate!"].concat()
            }
            CheckmateState::Check => [player_str, " is in check!"].concat(),
            CheckmateState::Normal => [player_str, " to move."].concat(),
        };
        self.status.text = text(status_text, ext_ctx.font, 25.0);

        // Update buttons
        use UIState::*;
        match self.ui_state() {
            Normal => (),
            GameOver => {
                self.main_menu.upd8(ctx);
                self.restart.upd8(ctx);
            }
            Promote(_) => {
                for (button, _) in &mut self.promote_buttons {
                    button.upd8(ctx);
                }
            }
        }

        // TODO: It is probably wasteful to relayout every frame. Maybe every turn?
        self.relayout(ext_ctx);
    }

    fn mouse_down_upd8(&mut self, mouse_pos: mint::Point2<f32>) {
        let coord = self.to_grid_coord(mouse_pos);
        match coord {
            Err(_) => {
                self.drop_locations = vec![];
            }
            Ok(coord) => {
                self.drop_locations = self.board.get_move_list(coord);
            }
        };
    }

    fn mouse_up_upd8(
        &mut self,
        ctx: &mut Context,
        mouse: &MouseState,
        screen_state: &mut ScreenState,
    ) {
        use UIState::*;
        match self.ui_state() {
            Normal => self.move_piece(ctx, mouse),
            GameOver => {
                if self.restart.pressed(mouse.pos) {
                    self.new_game();
                }

                if self.main_menu.pressed(mouse.pos) {
                    *screen_state = ScreenState::TitleScreen;
                }
            }
            Promote(coord) => {
                for (button, piece) in &self.promote_buttons {
                    if button.pressed(mouse.pos) {
                        self.board
                            .promote(coord, *piece)
                            .expect("Expected promotion to work");
                    }
                }
            }
        }
        self.drop_locations = vec![];
    }

    fn draw(&self, ctx: &mut Context, mouse: &MouseState, font: graphics::Font) -> GameResult<()> {
        graphics::draw(ctx, &self.background_mesh, (na::Point2::from(self.offset),))?;
        if self.ui_state() == UIState::Normal {
            self.draw_highlights(ctx, mouse)?;
        }

        self.draw_pieces(ctx, font)?;

        // Draw UI buttons, if applicable
        use UIState::*;
        match self.ui_state() {
            Normal => (),
            GameOver => {
                self.restart.draw(ctx, font)?;
                self.main_menu.draw(ctx, font)?;
            }
            Promote(_) => {
                for (button, _) in &self.promote_buttons {
                    button.draw(ctx, font)?;
                }
            }
        }

        self.status.draw(ctx)?;

        Ok(())
    }

    /// Call on mouse up
    /// Try to move the piece at the location of the last mouse down press to where the
    /// mouse currently is. A drag isn't being done, this function does nothing.
    fn move_piece(&mut self, ctx: &mut Context, mouse: &MouseState) {
        let dragging = self.to_grid_coord(mouse.last_down.unwrap());
        let drop_loc = self.to_grid_coord(mouse.pos);
        if drop_loc.is_err() || dragging.is_err() {
            return;
        }
        let drop_loc = drop_loc.unwrap();
        let dragging = dragging.unwrap();
        println!("{:?}", self.board.take_turn(dragging, drop_loc));
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

    fn draw_highlights(&self, ctx: &mut Context, mouse: &MouseState) -> GameResult<()> {
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

        // Color the potential locations the piece may be moved to in blue
        for coord in &self.drop_locations {
            let offset: na::Point2<f32> = self.to_screen_coord(*coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, BLUE))?;
        }

        // Color the currently highlighted square red if it is the player's piece or if the player is dragging it
        let pos = self.to_grid_coord(mouse.pos).ok();
        if let Some(coord) = pos {
            let same_color = self.board.get(coord).is_color(self.board.current_player);
            let is_dragging = mouse.dragging.is_some();
            let color = if same_color || is_dragging {
                RED
            } else {
                TRANSPARENT
            };
            let offset: na::Point2<f32> = self.to_screen_coord(coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, color))?;
        }

        // Color the dragged square green
        if let Some(dragging) = mouse.dragging {
            let dragging = self.to_grid_coord(dragging);
            if let Ok(coord) = dragging {
                let offset: na::Point2<f32> = self.to_screen_coord(coord) + self.offset;
                graphics::draw(ctx, &hollow_rect, (offset, GREEN))?;
            }
        }

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
    pub hitbox: Rect,
    state: ButtonState,
    text: graphics::Text,
}

impl Button {
    fn new(hitbox: Rect, text: graphics::Text) -> Button {
        Button {
            hitbox,
            state: ButtonState::Idle,
            text,
        }
    }

    /// Return a button whose size is at least large enough to fit both min_hitbox
    /// and the text. If the text would be larger than min_hitbox, it is centered on top of
    /// min_hitbox.
    fn fit_to_text(ctx: &mut Context, min_hitbox: Rect, text: Text) -> Button {
        let (w, h) = text.dimensions(ctx);
        let text_hitbox = center_inside(
            min_hitbox,
            Rect::new(min_hitbox.x, min_hitbox.y, w as f32, h as f32),
        );
        let hitbox = text_hitbox.combine_with(min_hitbox);

        Button::new(hitbox, text)
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
        draw_centered(ctx, &self.text, (location, BLACK))?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct TitleScreen {
    start_game: Button,
    quit_game: Button,
}

impl TitleScreen {
    fn new(ctx: &mut Context, font: graphics::Font) -> TitleScreen {
        TitleScreen {
            start_game: Button::fit_to_text(
                ctx,
                center(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT * 0.5, 300., 35.0),
                text("Start Game", font, 30.0),
            ),
            // start_game: Button::new(
            //     center(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT * 0.50, 300.0, 35.0),
            //     "Start Game",
            // ),
            quit_game: Button::fit_to_text(
                ctx,
                center(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT * 0.6, 300.0, 35.0),
                text("Quit Game", font, 30.0),
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

#[derive(Clone, Debug)]
pub struct TextBox {
    bounding_box: Rect,
    text: Text,
}

impl TextBox {
    fn empty() -> TextBox {
        TextBox {
            bounding_box: Rect::default(),
            text: Text::default(),
        }
    }
    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        let dims = (
            self.text.dimensions(ctx).0 as f32,
            self.text.dimensions(ctx).1 as f32,
        );
        let text_offset = center_inside(self.bounding_box, from_dims(dims).into());

        graphics::draw(ctx, &self.text, (text_offset.point(), RED))
    }
}

impl Layout for TextBox {
    fn bounding_box(&self) -> Rect {
        self.bounding_box
    }

    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        self.bounding_box.layout(max_size)
    }

    fn set_position(&mut self, pos: ggez::mint::Point2<f32>) {
        self.bounding_box.move_to(pos);
    }
    fn set_position_relative(&mut self, offset: ggez::mint::Vector2<f32>) {
        self.bounding_box.translate(offset);
    }
    fn preferred_size(&self) -> Option<(f32, f32)> {
        self.bounding_box.preferred_size()
    }
}

// Evenly distribute a number of `goal_size` Rect inside of `bounding_box`.
fn distribute_horiz(num_rects: u32, bounding_box: Rect, goal_size: Rect) -> Vec<Rect> {
    let bounding_boxes = divide_horiz(num_rects, bounding_box);
    let rects = bounding_boxes
        .iter()
        .map(|bounding| center_inside(*bounding, goal_size));

    rects.collect()
}

// Evenly divide bounding_box into `num_rects` smaller rects, horizontally.
fn divide_horiz(num_rects: u32, bounding_box: Rect) -> Vec<Rect> {
    let offset_x = bounding_box.x;
    let offset_y = bounding_box.y;
    let width = bounding_box.w / num_rects as f32;
    let height = bounding_box.h;
    let mut rects = vec![];
    for i in 0..num_rects {
        rects.push(Rect::new(
            i as f32 * width + offset_x,
            offset_y,
            width,
            height,
        ));
    }
    rects
}

// Aligns the inner rect to the bottom of the outer rect
fn align_bottom(outer: Rect, inner: Rect) -> Rect {
    let outer_bottom = outer.y + outer.h;
    Rect::new(inner.x, outer_bottom - inner.h, inner.w, inner.h)
}

fn from_points(start_x: f32, start_y: f32, end_x: f32, end_y: f32) -> Rect {
    Rect::new(start_x, start_y, end_x - start_x, end_y - start_y)
}

fn text<T>(text: T, font: graphics::Font, scale: f32) -> Text
where
    T: Into<graphics::TextFragment>,
{
    let mut text = graphics::Text::new(text);
    text.set_font(font, graphics::Scale::uniform(scale));
    text
}

fn draw_centered<D, S>(ctx: &mut Context, mesh: &D, params: S) -> GameResult<()>
where
    S: Into<DrawParam>,
    D: graphics::Drawable,
{
    let params: DrawParam = params.into();

    // Find the point such that the mesh will be centered on `param.dest`
    let dimensions = mesh.dimensions(ctx).unwrap();
    let (width, height) = (dimensions.w, dimensions.h);
    let location = mint::Point2 {
        x: params.dest.x - width as f32 / 2.0,
        y: params.dest.y - height as f32 / 2.0,
    };
    let params = params.dest(location);

    graphics::draw(ctx, mesh, params)
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
