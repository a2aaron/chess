use std::collections::{BinaryHeap, HashMap};
use std::time::{Duration, Instant};

use rand::Rng;

use ggez::event::{EventHandler, MouseButton};
use ggez::graphics::{self, DrawParam, Rect, Text};
use ggez::mint;
use ggez::nalgebra as na;
use ggez::{Context, GameResult};

use chess::ai::{AIPlayer, RandomPlayer, TreeSearchPlayer};
use chess::board::{
    move_type_coords, Board, BoardCoord, BoardState, CheckmateState, Color, MoveTypeCoords, Piece,
    PieceType, BISHOP_STR, KNIGHT_STR, QUEEN_STR, ROOK_STR,
};
use chess::color;
use chess::ease;
use chess::layout::{FlexBox, HStack, Layout, VStack};
use chess::particle;
use chess::rect;
use chess::ui::{self, Button, Selector, TextBox};
use chess::{hstack, vstack};

const PI: f32 = std::f32::consts::PI;

pub const SCREEN_WIDTH: f32 = 800.0;
pub const SCREEN_HEIGHT: f32 = 600.0;

const DEBUG_RESTART: bool = true;

const DEFAULT_SCALE: f32 = 20.0;
const DONTCARE: f32 = -999.0;

const MIN_TIME_BETWEEN_MOVES: f32 = 1.0;
const DEFAULT_ANIMATION_LENGTH: f32 = 0.22;
const DEFAULT_PREDELAY: f32 = 0.3;
const HARD_AI_MAX_DEPTH: usize = 6;

/// The entire game struct. This struct implements ggez's `EventHandler` and
/// orchestrates howeverthing should work.
#[derive(Debug)]
pub struct Game {
    /// which screen is currently active
    screen: ScreenState,
    /// which screen to transition to
    transition: ScreenTransition,
    title_screen: TitleScreen,
    grid: Grid,
    /// The extended context, containing image, font, and mouse data
    ext_ctx: ExtendedContext,
}

impl Game {
    /// Create a new Game from the context. This loads a font.
    pub fn new(ctx: &mut Context) -> Game {
        let font = graphics::Font::new(ctx, std::path::Path::new("\\freeserif.ttf")).unwrap();
        let mut ext_ctx = ExtendedContext::new(ctx, font);

        Game {
            screen: ScreenState::TitleScreen,
            transition: ScreenTransition::None,
            title_screen: TitleScreen::new(ctx, font),
            grid: Grid::new(ctx, &mut ext_ctx),
            ext_ctx,
        }
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // If we need to do some screen transition, then do it
        match &mut self.transition {
            ScreenTransition::StartGame(ai_white, ai_black) => {
                self.grid.set_ais(ai_white.take(), ai_black.take());
                self.grid.new_game();
                self.screen = ScreenState::InGame;
            }
            ScreenTransition::ToTitleScreen => {
                self.screen = ScreenState::TitleScreen;
            }
            ScreenTransition::QuitGame => ggez::event::quit(ctx),
            ScreenTransition::None => (),
        }
        // Now that we have done it, clear the transition state
        self.transition = ScreenTransition::None;

        match self.screen {
            ScreenState::TitleScreen => self.title_screen.upd8(ctx),
            ScreenState::InGame => self.grid.upd8(ctx, &mut self.ext_ctx),
        }

        self.ext_ctx.mouse_state.pos = ggez::input::mouse::position(ctx);

        for particle_sys in &mut self.ext_ctx.particles {
            particle_sys.upd8();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);
        // Mouse cursor
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
                self.grid.draw(ctx, &self.ext_ctx)?; // Draw particles in ext_ctx
                for particle_sys in &mut self.ext_ctx.particles {
                    particle_sys.draw(ctx)?;
                }
                self.grid.draw_pieces(ctx, &self.ext_ctx)?;
            }
        }

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

        // FPS counter
        let text = format!("{:.0}", ggez::timer::fps(ctx));
        let location = na::Point2::new(SCREEN_WIDTH - 20.0, 0.0);
        ui::draw_text(
            ctx,
            text,
            self.ext_ctx.font,
            DEFAULT_SCALE,
            (location, color::RED),
        )?;

        // Draw the mouse cursor. Note that this is last so that we draw the
        // mouse cursor over everything else
        graphics::draw(ctx, &circle, (self.ext_ctx.mouse_state.pos,))?;

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
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {
        // self.mouse_state.last_down = None;
        self.ext_ctx.mouse_state.last_up = Some(mint::Point2 { x, y });
        self.ext_ctx.mouse_state.dragging = None;

        match self.screen {
            ScreenState::TitleScreen => self
                .title_screen
                .mouse_up_upd8(mint::Point2 { x, y }, &mut self.transition),
            ScreenState::InGame => {
                self.grid
                    .mouse_up_upd8(ctx, &mut self.ext_ctx, &mut self.transition)
            }
        }
    }
}

/// This struct acts similarly to Context. It stores information that there should
/// normally only be one of (such as fonts, mouse state, etc) and provides
/// some useful debugging features.
#[derive(Debug)]
pub struct ExtendedContext {
    mouse_state: MouseState,
    /// font for in game text
    font: graphics::Font,
    particles: Vec<particle::ParticleSystem>,
    /// Debug rectangles. These are rendered in the chosen color on top of
    /// everything else.
    debug_render: Vec<(Rect, graphics::Color)>,
}

impl ExtendedContext {
    fn new(ctx: &mut Context, font: graphics::Font) -> ExtendedContext {
        ExtendedContext {
            mouse_state: MouseState::new(ctx),
            font,
            particles: vec![],
            debug_render: vec![],
        }
    }
}

/// An extension to ggez's MouseContext. It stores information such as the last
/// mouse down and mouse up positions, as well as the current "drag" of the mouse.
#[derive(Debug)]
pub struct MouseState {
    // The position of the last mouse down, if it exists
    last_down: Option<mint::Point2<f32>>,
    // The position of the last mouse up, if it exists
    last_up: Option<mint::Point2<f32>>,
    // Some(coord) if the mouse is pressed, else None
    dragging: Option<mint::Point2<f32>>,
    // The current position of the mouse
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

#[derive(Debug)]
pub struct TitleScreen {
    start_game: Button,
    white_selector: Selector,
    black_selector: Selector,
    quit_game: Button,
    title: TextBox,
}

impl TitleScreen {
    fn new(ctx: &mut Context, font: graphics::Font) -> TitleScreen {
        let mut title = TextBox::fit_to_text(ctx, text("CHESS", font, 60.0));
        let mut upper_padding = rect::from_dims((1.0, SCREEN_HEIGHT * 0.10));

        let buttons: Vec<Button> = vec![
            text("Human", font, 30.0),
            text("Easy AI", font, 30.0),
            text("Hard AI", font, 30.0),
        ]
        .into_iter()
        .map(|text| Button::fit_to_text(ctx, (100.0, 35.0), text))
        .collect();

        let mut black_selector = Selector::new(buttons.clone());
        let mut white_selector = Selector::new(buttons);

        let mut white_selector_stack = VStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut white_selector.buttons,
            min_dimensions: (None, None),
        };

        let mut black_selector_stack = VStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut black_selector.buttons,
            min_dimensions: (None, None),
        };

        // Unfortunately, Rust can't seem to infer the right type when the type
        // is &mut dyn Layout for some reason, so we say it explicitly.
        let mut selector_stack: HStack<&mut dyn Layout> = hstack! {
            Some(SCREEN_WIDTH), None =>
            FlexBox::new(1.0);
            white_selector_stack;
            rect::from_dims((30.0, 1.0));
            black_selector_stack;
            FlexBox::new(1.0);
        };

        let mut padding = rect::from_dims((1.0, SCREEN_HEIGHT * 0.10));
        let mut start_game =
            Button::fit_to_text(ctx, (300.0, 35.0), text("Start Game", font, 30.0));
        let mut quit_game = Button::fit_to_text(ctx, (300.0, 35.0), text("Quit Game", font, 30.0));
        let mut padding2 = rect::from_dims((1.0, 25.0));

        let mut vstack: VStack<&mut dyn Layout> = vstack! {
            Some(SCREEN_WIDTH), None =>
            title;
            upper_padding;
            selector_stack;
            padding;
            start_game;
            padding2;
            quit_game;
        };

        vstack.layout(vstack.preferred_size().unwrap());
        vstack.set_position_relative(mint::Vector2 {
            x: 0.0,
            y: SCREEN_HEIGHT * 0.15,
        });
        TitleScreen {
            title,
            white_selector,
            black_selector,
            start_game,
            quit_game,
        }
    }

    fn upd8(&mut self, ctx: &mut Context) {
        self.start_game.upd8(ctx);
        self.quit_game.upd8(ctx);
        self.white_selector.upd8(ctx);
        self.black_selector.upd8(ctx);
    }

    fn mouse_up_upd8(
        &mut self,
        mouse_pos: mint::Point2<f32>,
        screen_transition: &mut ScreenTransition,
    ) {
        // On game start, get which AIs should be used
        if self.start_game.pressed(mouse_pos) {
            let white_ai: Option<Box<dyn AIPlayer>> = match self.white_selector.selected {
                0 => None,
                1 => Some(Box::new(RandomPlayer {})),
                2 => Some(Box::new(TreeSearchPlayer::new(HARD_AI_MAX_DEPTH))),
                _ => unreachable!(),
            };

            let black_ai: Option<Box<dyn AIPlayer>> = match self.black_selector.selected {
                0 => None,
                1 => Some(Box::new(RandomPlayer {})),
                2 => Some(Box::new(TreeSearchPlayer::new(HARD_AI_MAX_DEPTH))),
                _ => unreachable!(),
            };
            *screen_transition = ScreenTransition::StartGame(white_ai, black_ai);
        }

        if self.quit_game.pressed(mouse_pos) {
            *screen_transition = ScreenTransition::QuitGame;
        }
    }

    fn draw(&self, ctx: &mut Context, _font: graphics::Font) -> GameResult<()> {
        self.title.draw(ctx)?;
        self.white_selector.draw(ctx)?;
        self.black_selector.draw(ctx)?;
        self.start_game.draw(ctx)?;
        self.quit_game.draw(ctx)?;

        Ok(())
    }
}

/// Which screen the game should transition to.
#[derive(Debug)]
pub enum ScreenTransition {
    None,
    StartGame(Option<Box<dyn AIPlayer>>, Option<Box<dyn AIPlayer>>),
    ToTitleScreen,
    QuitGame,
}

/// Which screen the game is current on.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ScreenState {
    TitleScreen,
    InGame,
}

#[derive(Debug)]
pub struct Grid {
    // Handles drawing the grid stuff
    grid: BoardView,
    // The actual underling board that runs the chess board.
    board: BoardState,
    // If this is None, then use a human player. Otherwise, use the listed AI player
    ai_black: Option<Box<dyn AIPlayer>>,
    ai_white: Option<Box<dyn AIPlayer>>,
    time_since_last_move: f32,
    // Handles drawing the sidebar UI
    sidebar: GameSidebar,
}

impl Grid {
    fn new(ctx: &mut Context, ext_ctx: &mut ExtendedContext) -> Grid {
        use PieceType::*;
        let square_size = 70.0;
        let font = ext_ctx.font;

        let button_size = (40.0, 35.0);
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
        let board = BoardState::new(Board::default());
        let offset: na::Vector2<f32> = na::Vector2::new(DONTCARE, DONTCARE);
        let mut grid = Grid {
            grid: BoardView {
                square_size,
                offset,
                drop_locations: vec![],
                animated_board: AnimatedBoard::new(&board, square_size, offset),
                background_mesh: BoardView::background_mesh(ctx, square_size),
                last_move: None,
            },
            board,
            ai_black: None,
            ai_white: None,
            time_since_last_move: 0.0,
            sidebar: GameSidebar {
                restart: Button::fit_to_text(
                    ctx,
                    (100.0, 35.0),
                    text("Restart Game", font, DEFAULT_SCALE),
                ),
                main_menu: Button::fit_to_text(
                    ctx,
                    (100.0, 35.0),
                    text("Main Menu", font, DEFAULT_SCALE),
                ),
                status: TextBox::new((110.0, 100.0)),
                promote_buttons,
                dead_black_list: vec![],
                dead_white_list: vec![],
                dead_black: TextBox::new((110.0, 100.0)),
                dead_white: TextBox::new((110.0, 100.0)),
            },
        };
        grid.relayout(ext_ctx);
        grid
    }

    fn set_ais(
        &mut self,
        ai_white: Option<Box<dyn AIPlayer>>,
        ai_black: Option<Box<dyn AIPlayer>>,
    ) {
        self.ai_white = ai_white;
        self.ai_black = ai_black;
    }

    fn relayout(&mut self, _ext_ctx: &mut ExtendedContext) {
        let off_x = 10.0;
        let off_y = 10.0;

        let button_size = self.sidebar.promote_buttons[0].0.hitbox;
        let mut layout_buttons = vec![];
        for (button, _) in self.sidebar.promote_buttons.iter_mut() {
            layout_buttons.push(button);
        }

        let mut grid = Rect::new(
            off_x,
            off_y,
            8.0 * self.grid.square_size,
            8.0 * self.grid.square_size,
        );
        let mut button_stack = HStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut layout_buttons,
            min_dimensions: (None, None),
        };

        let mut menu_buttons = vstack! {
            None, None =>
            self.sidebar.restart;
            self.sidebar.main_menu;
        };

        // Same size as the buttons, but used as padding
        let mut fake_stack = hstack! {
            None, None =>
            button_size.clone();
            button_size.clone();
            button_size.clone();
            button_size.clone();
        };

        let mut padding1 = FlexBox::new(1.0);
        let mut padding2 = FlexBox::new(2.0);
        let mut padding3 = FlexBox::new(2.0);
        let mut padding4 = FlexBox::new(1.0);
        let mut sidebar_children: [&mut dyn Layout; 10] = match self.board.current_player {
            Color::White => [
                &mut fake_stack,
                &mut padding1,
                &mut self.sidebar.dead_black,
                &mut padding2,
                &mut self.sidebar.status,
                &mut menu_buttons,
                &mut padding3,
                &mut self.sidebar.dead_white,
                &mut padding4,
                &mut button_stack,
            ],
            Color::Black => [
                &mut button_stack,
                &mut padding1,
                &mut self.sidebar.dead_black,
                &mut padding2,
                &mut self.sidebar.status,
                &mut menu_buttons,
                &mut padding3,
                &mut self.sidebar.dead_white,
                &mut padding4,
                &mut fake_stack,
            ],
        };

        let mut sidebar = VStack {
            pos: mint::Point2 { x: 0.0, y: 0.0 },
            children: &mut sidebar_children,
            min_dimensions: (Some(SCREEN_WIDTH - grid.right() - 10.0), Some(grid.h)),
        };

        let mut padding_side = FlexBox::new(1.0);
        let mut full_ui: HStack<&mut dyn Layout> = hstack! {
            Some(SCREEN_WIDTH - 20.0), None =>
            grid;
            padding_side;
            sidebar;
        };

        full_ui.layout((SCREEN_WIDTH - 20.0, SCREEN_HEIGHT - 20.0));

        full_ui.set_position_relative(mint::Vector2 { x: 10.0, y: 10.0 });

        self.grid.offset = na::Vector2::new(grid.x, grid.y);
        self.grid.animated_board.offset = na::Vector2::new(grid.x, grid.y);
    }

    fn new_game(&mut self) {
        // let board = vec![
        //     "BR .. .. .. BK .. .. BR",
        //     "BP BP BP BP BP BP BP BP",
        //     ".. .. .. .. .. .. .. ..",
        //     ".. .. .. .. .. .. .. ..",
        //     ".. .. .. .. .. .. .. ..",
        //     ".. .. .. .. .. .. .. ..",
        //     "WP WP WP WP WP WP WP WP",
        //     "WR .. .. .. WK .. .. WR",
        // ];
        // let board = Board::from_string_vec(board);
        let board = Board::default();
        self.board = BoardState::new(board);
        self.time_since_last_move = 0.0;
        self.grid.new_game(&self.board);
    }

    fn upd8(&mut self, ctx: &mut Context, ext_ctx: &mut ExtendedContext) {
        self.sidebar.upd8(ctx, ext_ctx, &self.board);

        // Take AI turn, if it isn't game over and it has been at least MIN_TIME_BETWEEN_MOVES
        if !self.board.game_over() {
            let ai = match self.board.current_player {
                Color::White => &mut self.ai_white,
                Color::Black => &mut self.ai_black,
            };
            // If we have an AI and the AI is ready, take the move if we have waited some
            // minimum time. This is done to limit fast AIs from spam moving
            if let Some(ai) = ai {
                if let std::task::Poll::Ready((start, end)) =
                    ai.next_move(&self.board, self.board.current_player)
                {
                    if self.time_since_last_move >= MIN_TIME_BETWEEN_MOVES {
                        self.board
                            .check_turn(start, end)
                            .expect("AI made an illegal move");
                        self.take_turn(ctx, start, end);
                    }
                }
            }
            // We have to do this reborrow here because rust can't seem to figure out
            // the lifetimes otherwise (it complains that reusing `ai` down here would
            // result in a double mutable borrow, so we do this to end the lifetime of
            // `ai` early)
            let ai = match self.board.current_player {
                Color::White => &mut self.ai_white,
                Color::Black => &mut self.ai_black,
            };
            if let Some(ai) = ai {
                // If this move would require the AI to promote a piece, then ask
                // the AI to promote the piece.
                if let Some(coord) = self.board.need_promote() {
                    let piece = ai.next_promote(&self.board);
                    if let std::task::Poll::Ready(piece) = piece {
                        self.board
                            .check_promote(coord, piece)
                            .expect("AI made an illegal promote");
                        Self::promote(&mut self.board, &mut self.grid, coord, piece);
                    }
                }
            }
        }

        self.grid.upd8(ctx, ext_ctx);

        // TODO: It is probably wasteful to relayout every frame. Maybe every turn?
        self.relayout(ext_ctx);

        self.time_since_last_move += ggez::timer::delta(ctx).as_secs_f32();
    }

    fn mouse_down_upd8(&mut self, mouse_pos: mint::Point2<f32>) {
        if self.current_player_is_human() {
            self.grid.upd8_drop_locations(mouse_pos, &self.board)
        }
    }

    fn mouse_up_upd8(
        &mut self,
        ctx: &mut Context,
        ext_ctx: &mut ExtendedContext,
        transition: &mut ScreenTransition,
    ) {
        let mouse = &ext_ctx.mouse_state;
        use UIState::*;
        match self.ui_state() {
            Normal => {
                if self.current_player_is_human() {
                    // On a mouse up, try moving the held piece to the current mouse position
                    let dragging = self.grid.to_grid_coord(mouse.last_down.unwrap());
                    let drop_loc = self.grid.to_grid_coord(mouse.pos);
                    // If both the drag point and drop point are within the grid
                    if let (Ok(start), Ok(end)) = (dragging, drop_loc) {
                        if self.board.check_turn(start, end).is_ok() {
                            // We don't ratelimit how fast humans can move since it's really unlikely they'll
                            // move too fast for the other player to see
                            self.take_turn(ctx, start, end);
                        }
                    }
                }
            }
            GameOver => {
                if self.sidebar.restart.pressed(mouse.pos) {
                    ext_ctx.particles.clear();
                    self.new_game();
                }

                if self.sidebar.main_menu.pressed(mouse.pos) {
                    ext_ctx.particles.clear();
                    *transition = ScreenTransition::ToTitleScreen;
                }
            }
            Promote(coord) => {
                for (button, piece) in &self.sidebar.promote_buttons {
                    if button.pressed(mouse.pos) {
                        Self::promote(&mut self.board, &mut self.grid, coord, *piece);
                    }
                }
            }
        }
        if DEBUG_RESTART && self.sidebar.restart.pressed(mouse.pos) {
            ext_ctx.particles.clear();
            self.new_game();
        }
        self.grid.drop_locations = vec![];
    }

    fn promote(board: &mut BoardState, grid: &mut BoardView, coord: BoardCoord, piece: PieceType) {
        grid.promote(coord, piece);
        board.promote(coord, piece);
    }

    // Move the piece from start to end and update the last move/animation boards
    fn take_turn(&mut self, _ctx: &mut Context, start: BoardCoord, end: BoardCoord) {
        // In the event of a capture, add the piece to the appropriate list
        match move_type_coords(&self.board.board, start, end) {
            MoveTypeCoords::Capture { end: capture, .. }
            | MoveTypeCoords::EnPassant {
                captured_pawn: capture,
                ..
            } => {
                let piece = self
                    .board
                    .get(capture)
                    .0
                    .expect("Expected capture to have a piece");
                match piece.color {
                    Color::Black => self.sidebar.dead_black_list.push(piece.piece),
                    Color::White => self.sidebar.dead_white_list.push(piece.piece),
                }
            }
            _ => (),
        }

        // Update the view first here because we want it to work off of the state
        // of board _before_ we make the actual move
        self.grid.take_turn(&self.board, start, end);
        self.board.take_turn(start, end);

        // Set the time since the last move so the AI does not move immediately.
        self.time_since_last_move = 0.0;
    }

    fn draw(&self, ctx: &mut Context, ext_ctx: &ExtendedContext) -> GameResult<()> {
        graphics::draw(
            ctx,
            &self.grid.background_mesh,
            (na::Point2::from(self.grid.offset),),
        )?;

        if self.ui_state() == UIState::Normal {
            let mouse = &ext_ctx.mouse_state;
            self.grid.draw_highlights(ctx, mouse, &self.board)?;
        }

        self.sidebar.draw(ctx, self.ui_state())
    }

    fn draw_pieces(&self, ctx: &mut Context, ext_ctx: &ExtendedContext) -> GameResult<()> {
        self.grid.animated_board.draw(ctx, ext_ctx)
    }

    /// Returns true if the current player is a human player
    fn current_player_is_human(&self) -> bool {
        match self.board.current_player {
            Color::White => self.ai_white.is_none(),
            Color::Black => self.ai_black.is_none(),
        }
    }

    /// Get the current UIState based on if it's game over or if a piece needs to be promoted
    fn ui_state(&self) -> UIState {
        match (self.board.game_over(), self.board.need_promote()) {
            (false, None) => UIState::Normal,
            (false, Some(coord)) => UIState::Promote(coord),
            (true, _) => UIState::GameOver,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum UIState {
    Normal,
    Promote(BoardCoord),
    GameOver,
}

#[derive(Debug)]
struct BoardView {
    // Size of a single square, in pixels
    square_size: f32,
    // Offset of the entire screen from the upper left.
    offset: na::Vector2<f32>,
    // The list of locations the currently held piece can be placed. If this
    // vector is empty, then either no piece is being held or there are no places
    // to move that piece
    drop_locations: Vec<BoardCoord>,
    // The pieces meant to be drawn to the board. This struct controls the animations
    // of moving pieces.
    animated_board: AnimatedBoard,
    // If Some, then this will contain the previous move just made. This is used
    // to highlight the "just moved" piece.
    last_move: Option<(BoardCoord, BoardCoord)>,
    // The checkerboard background of the board
    background_mesh: graphics::Mesh,
}

/// This struct mostly handles drawing the chess board, its pieces, square
/// highlights, and handling screenspace/boardspace convesions
impl BoardView {
    fn new_game(&mut self, board: &BoardState) {
        self.drop_locations = vec![];
        self.last_move = None;
        self.animated_board = AnimatedBoard::new(board, self.square_size, self.offset);
    }

    fn upd8(&mut self, ctx: &mut Context, ext_ctx: &mut ExtendedContext) {
        // probably another thing here????
        self.animated_board.upd8(ctx, ext_ctx);
    }

    fn upd8_drop_locations(&mut self, mouse_pos: mint::Point2<f32>, board: &BoardState) {
        let coord = to_grid_coord(self.square_size, self.offset, mouse_pos);
        match coord {
            Err(_) => {
                self.drop_locations = vec![];
            }
            Ok(coord) => {
                self.drop_locations = board.get_move_list(coord);
            }
        };
    }

    /// Draw the board highlights (current tile selected, places to move, etc)
    fn draw_highlights(
        &self,
        ctx: &mut Context,
        mouse: &MouseState,
        board: &BoardState,
    ) -> GameResult<()> {
        let fill: graphics::DrawMode = graphics::DrawMode::fill();

        let mut mesh = graphics::MeshBuilder::new();
        let solid_rect = Rect::new(0.0, 0.0, self.square_size, self.square_size);
        mesh.rectangle(fill, solid_rect, color::WHITE);
        let solid_rect = mesh.build(ctx).unwrap();

        // Highlight  the "last moved" squares in transparent green
        if let Some((start, end)) = self.last_move {
            let start = self.to_screen_coord(start) + self.offset;
            let end = self.to_screen_coord(end) + self.offset;
            const VERY_TRANS_GREEN: graphics::Color = graphics::Color::new(0.0, 1.0, 0.0, 0.3);
            graphics::draw(ctx, &solid_rect, (start, VERY_TRANS_GREEN))?;
            graphics::draw(ctx, &solid_rect, (end, VERY_TRANS_GREEN))?;
        }

        // TODO: this is an awful idea, instead expose a field similar to self.board.checkmate
        let king_coord = board.board.get_king(board.current_player);

        // If king in check, draw it in red.
        if let Some(coord) = king_coord {
            if board.checkmate != CheckmateState::Normal {
                let offset = self.to_screen_coord(coord) + self.offset;
                graphics::draw(ctx, &solid_rect, (offset, color::RED))?;
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
        // the color here is not relevant, as it will be overriden below
        mesh.rectangle(stroke, hollow_rect, color::WHITE);
        let hollow_rect = mesh.build(ctx).unwrap();

        // Color the potential locations the piece may be moved to in blue
        for coord in &self.drop_locations {
            let offset: na::Point2<f32> = self.to_screen_coord(*coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, color::BLUE))?;
        }

        // Color the currently highlighted square red if it is the player's piece or if the player is dragging it
        let pos = to_grid_coord(self.square_size, self.offset, mouse.pos).ok();
        if let Some(coord) = pos {
            let same_color = board.get(coord).is_color(board.current_player);
            let is_dragging = mouse.dragging.is_some();
            let color = if same_color || is_dragging {
                color::RED
            } else {
                color::TRANSPARENT
            };
            let offset: na::Point2<f32> = self.to_screen_coord(coord) + self.offset;
            graphics::draw(ctx, &hollow_rect, (offset, color))?;
        }

        // Color the dragged square green
        if let Some(dragging) = mouse.dragging {
            let dragging = to_grid_coord(self.square_size, self.offset, dragging);
            if let Ok(coord) = dragging {
                let offset: na::Point2<f32> = self.to_screen_coord(coord) + self.offset;
                graphics::draw(ctx, &hollow_rect, (offset, color::GREEN))?;
            }
        }

        Ok(())
    }

    /// Construct the checkerboard mesh background
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
                    mesh.rectangle(fill, rect, color::LIGHT_GREY);
                } else {
                    mesh.rectangle(fill, rect, color::DARK_GREY);
                }
            }
        }

        mesh.build(ctx).unwrap()
    }

    fn promote(&mut self, coord: BoardCoord, piece: PieceType) {
        self.last_move = Some((coord, coord));
        self.animated_board.promote(coord, piece);
    }

    // Move the piece from start to end and update the last move/animation boards
    fn take_turn(&mut self, board: &BoardState, start: BoardCoord, end: BoardCoord) {
        self.last_move = Some((start, end));
        self.animated_board.take_turn(board, start, end);
    }

    fn to_grid_coord(&self, screen_coords: mint::Point2<f32>) -> Result<BoardCoord, &'static str> {
        to_grid_coord(self.square_size, self.offset, screen_coords)
    }

    fn to_screen_coord(&self, board_coord: BoardCoord) -> na::Point2<f32> {
        na::Point2::from(to_screen_coord(self.square_size, board_coord))
    }
}

#[derive(Debug)]
struct AnimationEvent {
    // Only the variant here matters.
    action: AnimationType,
    // what piece to apply the action to
    id: usize,
    // How long this animation event takes place.
    animation_duration: f32,
    // When to fire this event.
    start_time: Instant,
}

impl PartialOrd for AnimationEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.start_time.cmp(&other.start_time).reverse())
    }
}

impl PartialEq for AnimationEvent {
    fn eq(&self, other: &Self) -> bool {
        self.start_time.eq(&other.start_time)
    }
}

impl Ord for AnimationEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // this is ok because Instant is Ord, so this cant fail
        self.partial_cmp(other).unwrap()
    }
}

impl Eq for AnimationEvent {}
#[derive(Debug)]
enum AnimationType {
    Move {
        target: BoardCoord,
    },
    Remove {
        coord: BoardCoord,
        angle: na::Vector2<f32>,
        intensity: f32,
        spread: f32,
        size: u16,
        num_particles: usize,
    },
    Change {
        piece: PieceType,
    },
}

#[derive(Debug)]
struct AnimatedBoard {
    square_size: f32,
    offset: na::Vector2<f32>,
    // A hashmap of all of the _alive_ pieces on the board and their current position
    // Note that this hash map stores indicies into `pieces`.
    coords: HashMap<BoardCoord, usize>,
    // The actual pieces. This vector should not be changed after initalization
    // Piece which end up "dead" should not be removed from this vector, instead
    // remove it from coords.
    pieces: Vec<AnimatedPiece>,
    event_queue: BinaryHeap<AnimationEvent>,
}

impl AnimatedBoard {
    fn new(board: &BoardState, square_size: f32, offset: na::Vector2<f32>) -> AnimatedBoard {
        let mut coords = HashMap::with_capacity(32);
        let mut pieces = Vec::with_capacity(32);
        for i in 0..8 {
            for j in 0..8 {
                let coord = BoardCoord::new((i, j)).expect("Expected a valid BoardCoord");
                if let Some(piece) = board.get(coord).0 {
                    let end = to_screen_coord(square_size, coord);
                    let start = match piece.color {
                        Color::Black => mint::Point2::<f32> {
                            x: end.x,
                            y: -100.0,
                        },
                        Color::White => mint::Point2::<f32> {
                            x: end.x,
                            y: SCREEN_HEIGHT + 100.0,
                        },
                    };
                    let piece = AnimatedPiece::new(
                        start,
                        end,
                        piece,
                        rand::thread_rng().gen_range(0.4, 0.8),
                    );
                    pieces.push(piece);
                    let id = pieces.len() - 1;
                    coords.insert(coord, id);
                }
            }
        }
        AnimatedBoard {
            square_size,
            offset,
            coords,
            pieces,
            event_queue: BinaryHeap::with_capacity(3),
        }
    }

    fn upd8(&mut self, ctx: &mut Context, ext_ctx: &mut ExtendedContext) {
        for piece in self.pieces.iter_mut().filter(|piece| piece.alive) {
            piece.upd8(ggez::timer::delta(ctx).as_secs_f32());
        }
        let now = Instant::now();
        match self.event_queue.peek() {
            Some(event) if event.start_time < now => {
                let event = self.event_queue.pop().unwrap();

                match event.action {
                    AnimationType::Move { target } => {
                        let target = self.to_screen_coord(target);
                        self.pieces[event.id].set_target(target);
                    }
                    AnimationType::Change { piece } => {
                        self.pieces[event.id].set_piecetype(piece);
                    }
                    AnimationType::Remove {
                        coord,
                        angle,
                        intensity,
                        spread,
                        size,
                        num_particles,
                    } => {
                        self.pieces[event.id].alive = false;
                        ext_ctx.particles.push(particle::ParticleSystem::new(
                            ctx,
                            self.to_screen_coord_centered(coord),
                            angle,
                            spread,
                            intensity,
                            size,
                            num_particles,
                        ));
                    }
                }
            }
            _ => (),
        }
    }

    fn draw(&self, ctx: &mut Context, ext_ctx: &ExtendedContext) -> GameResult<()> {
        for piece in self.pieces.iter().filter(|piece| piece.alive) {
            piece.draw(ctx, ext_ctx, self.square_size)?;
        }
        ui::draw_text_workaround(ctx);
        Ok(())
    }

    fn promote(&mut self, coord: BoardCoord, piece: PieceType) {
        self.event_queue.push(AnimationEvent {
            action: AnimationType::Change { piece },
            id: *self.coords.get(&coord).unwrap(),
            animation_duration: 0.0,
            start_time: Instant::now(),
        })
    }

    fn take_turn(&mut self, board: &BoardState, start: BoardCoord, end: BoardCoord) {
        fn move_event(
            coords: &HashMap<BoardCoord, usize>,
            start: BoardCoord,
            end: BoardCoord,
        ) -> AnimationEvent {
            AnimationEvent {
                action: AnimationType::Move { target: end },
                id: *coords.get(&start).unwrap(),
                animation_duration: DEFAULT_ANIMATION_LENGTH,
                start_time: Instant::now(),
            }
        }

        fn remove_event(
            board: &AnimatedBoard,
            start: BoardCoord,
            end: BoardCoord,
            intensity: f32,
            spread: f32,
            num_particles: usize,
        ) -> AnimationEvent {
            let screen_start = board.to_screen_coord_centered(start);
            let screen_end = board.to_screen_coord_centered(end);
            let angle = screen_end - screen_start;
            AnimationEvent {
                action: AnimationType::Remove {
                    coord: end,
                    angle,
                    intensity,
                    spread,
                    num_particles,
                    size: 6,
                },
                id: *board.coords.get(&end).unwrap(),
                animation_duration: DEFAULT_ANIMATION_LENGTH,
                start_time: Instant::now() + Duration::from_secs_f32(DEFAULT_ANIMATION_LENGTH),
            }
        }

        // Helper functions to modify the hashmap
        fn move_piece(coords: &mut HashMap<BoardCoord, usize>, start: BoardCoord, end: BoardCoord) {
            let id = coords
                .remove(&start)
                .expect("HashMap did not contain piece--this is probably a desync earlier on");
            coords.insert(end, id);
        }

        fn remove(coords: &mut HashMap<BoardCoord, usize>, coord: BoardCoord) {
            coords
                .remove(&coord)
                .expect("HashMap did not contain piece--this is probably a desync earlier on");
        }

        use MoveTypeCoords::*;
        // Update interal board representation
        match move_type_coords(&board.board, start, end) {
            Normal { start, end } | Lunge { start, end } => {
                let event = move_event(&self.coords, start, end);
                self.event_queue.push(event);
                move_piece(&mut self.coords, start, end);
            }
            Capture { start, end } => {
                let move_event = move_event(&self.coords, start, end);
                self.event_queue.push(move_event);
                let distance_moved = (self.to_screen_coord_centered(end)
                    - self.to_screen_coord_centered(start))
                .norm();
                let num_particles = (distance_moved / 4.0) as usize;
                let remove_event =
                    remove_event(&self, start, end, distance_moved, PI / 6.0, num_particles);
                self.event_queue.push(remove_event);
                move_piece(&mut self.coords, start, end);
            }
            Castle {
                king_start,
                king_end,
                rook_start,
                rook_end,
            } => {
                let king = move_event(&self.coords, king_start, king_end);
                let rook = move_event(&self.coords, rook_start, rook_end);
                self.event_queue.push(king);
                self.event_queue.push(rook);
                move_piece(&mut self.coords, king_start, king_end);
                move_piece(&mut self.coords, rook_start, rook_end);
            }
            EnPassant {
                start,
                end,
                captured_pawn,
            } => {
                let move_event = move_event(&self.coords, start, end);
                let remove_event = AnimationEvent {
                    action: AnimationType::Remove {
                        coord: captured_pawn,
                        angle: na::Vector2::new(1.0, 0.0),
                        intensity: 35.0,
                        spread: PI * 2.0,
                        num_particles: 25,
                        size: 6,
                    },
                    id: *self.coords.get(&captured_pawn).unwrap(),
                    animation_duration: DEFAULT_ANIMATION_LENGTH,
                    start_time: Instant::now() + Duration::from_secs_f32(DEFAULT_ANIMATION_LENGTH),
                };

                self.event_queue.push(move_event);
                self.event_queue.push(remove_event);

                move_piece(&mut self.coords, start, end);
                remove(&mut self.coords, captured_pawn);
            }
        }
    }

    fn to_screen_coord(&self, board_coord: BoardCoord) -> mint::Point2<f32> {
        to_screen_coord(self.square_size, board_coord)
    }

    fn to_screen_coord_centered(&self, coord: BoardCoord) -> na::Point2<f32> {
        na::Point2::from(self.to_screen_coord(coord))
            + na::Vector2::new(self.square_size / 2.0, self.square_size / 2.0)
    }
}

/// Returns a tuple of where the given screen space coordinates would end up
/// on the grid. This function returns Err if the point would be off the grid.
fn to_grid_coord<V: Into<na::Vector2<f32>>>(
    square_size: f32,
    offset: V,
    screen_coords: mint::Point2<f32>,
) -> Result<BoardCoord, &'static str> {
    let offset_coords = na::Point2::from(screen_coords) - offset.into();
    let grid_x = (offset_coords.x / square_size).floor() as i8;
    let grid_y = (offset_coords.y / square_size).floor() as i8;
    BoardCoord::new((grid_x, 7 - grid_y))
}

/// Returns the upper left corner of the square located at `board_coords`
fn to_screen_coord(square_size: f32, board_coord: BoardCoord) -> mint::Point2<f32> {
    mint::Point2 {
        x: board_coord.0 as f32 * square_size,
        y: (7 - board_coord.1) as f32 * square_size,
    }
}

#[derive(Debug)]
struct AnimatedPiece {
    alive: bool,
    piece: Piece,
    pos: mint::Point2<f32>,
    start: mint::Point2<f32>,
    end: mint::Point2<f32>,
    // how far into the animation this piece is. Should be reset to zero on
    // a new set target
    timer: f32,
    ani_length: f32,
    pre_delay: f32,
    ease: ease::Ease,
}

impl AnimatedPiece {
    fn new(
        start: mint::Point2<f32>,
        end: mint::Point2<f32>,
        piece: Piece,
        ani_length: f32,
    ) -> AnimatedPiece {
        AnimatedPiece {
            alive: true,
            pos: start,
            start,
            end,
            piece,
            timer: 0.0,
            ani_length,
            pre_delay: DEFAULT_PREDELAY,
            ease: ease::Ease::InOutBack,
        }
    }

    fn upd8(&mut self, dt: f32) {
        self.timer += dt;
        // limit percent to range [0.0, 1.0]
        let percent = ((self.timer - self.pre_delay) / self.ani_length).clamp(0.0, 1.0);
        self.pos.x = self.ease.interpolate(self.start.x, self.end.x, percent);
        self.pos.y = self.ease.interpolate(self.start.y, self.end.y, percent);
    }

    fn set_target(&mut self, target: mint::Point2<f32>) {
        self.start = self.pos;
        self.end = target;
        self.timer = 0.0;
        self.ani_length = DEFAULT_ANIMATION_LENGTH;
        self.pre_delay = 0.0;
        self.ease = ease::Ease::InOutCubic;
    }

    fn draw(
        &self,
        ctx: &mut Context,
        ext_ctx: &ExtendedContext,
        square_size: f32,
    ) -> GameResult<()> {
        let mut text = graphics::Text::new(self.piece.as_str());
        let text = text.set_font(ext_ctx.font, graphics::Scale::uniform(50.0));
        let location =
            na::Point2::from(self.pos) + na::Vector2::new(square_size * 0.42, square_size * 0.25);
        let color = match self.piece.color {
            Color::Black => color::BLACK,
            Color::White => color::WHITE,
        };
        graphics::draw(ctx, text, (location, color))
    }

    fn set_piecetype(&mut self, piece: PieceType) {
        self.piece.piece = piece;
    }
}

/// This struct handles the drawing and state maitence of the sidebar. Note that
/// this struct does not actually handle button functionality--this is done
/// up in Grid
#[derive(Debug)]
struct GameSidebar {
    // Restart and main menu buttons
    restart: Button,
    main_menu: Button,
    // Promotion buttons. Note that this is reused for both white's and black's side
    // and we just move the buttons around as needed. The PieceType tells what
    // piece the pawn will promote to.
    promote_buttons: Vec<(Button, PieceType)>,
    // Displays who's turn it is and if there is check/checkmate/etc or not
    status: TextBox,
    dead_black_list: Vec<PieceType>,
    dead_white_list: Vec<PieceType>,
    dead_black: TextBox,
    dead_white: TextBox,
}

impl GameSidebar {
    fn upd8(&mut self, ctx: &mut Context, ext_ctx: &mut ExtendedContext, board: &BoardState) {
        // Update status message
        let player_str = board.current_player.as_str();

        let status_text = match board.checkmate {
            CheckmateState::Stalemate => "The game has ended!\nStalemate!".to_owned(),
            CheckmateState::InsuffientMaterial => {
                "The game has ended!\nInsuffient material!".to_owned()
            }
            CheckmateState::Checkmate => {
                ["The game has ended!\n", player_str, " is in checkmate!"].concat()
            }
            CheckmateState::Check => [player_str, " is in check!"].concat(),
            CheckmateState::Normal => [player_str, " to move."].concat(),
        };
        self.status.text = text(status_text, ext_ctx.font, 25.0);

        self.dead_black.text = text(
            piece_slice_to_str(&self.dead_black_list),
            ext_ctx.font,
            30.0,
        );
        self.dead_white.text = text(
            piece_slice_to_str(&self.dead_white_list),
            ext_ctx.font,
            30.0,
        );

        // Update buttons
        self.main_menu.upd8(ctx);
        self.restart.upd8(ctx);

        for (button, _) in &mut self.promote_buttons {
            button.upd8(ctx);
        }
    }

    fn draw(&self, ctx: &mut Context, ui_state: UIState) -> GameResult<()> {
        // Draw UI buttons, if applicable
        use UIState::*;
        match ui_state {
            Normal => (),
            GameOver => {
                self.restart.draw(ctx)?;
                self.main_menu.draw(ctx)?;
            }
            Promote(_) => {
                for (button, _) in &self.promote_buttons {
                    button.draw(ctx)?;
                }
            }
        }

        self.status.draw(ctx)?;
        self.dead_black.draw(ctx)?;
        self.dead_white.draw(ctx)?;

        if DEBUG_RESTART {
            self.restart.draw(ctx)?;
        }

        Ok(())
    }
}

/// Convert a list of pieces to their string representation.
fn piece_slice_to_str(pieces: &[PieceType]) -> String {
    let mut string = String::new();
    for (i, piece) in pieces.iter().enumerate() {
        string.push_str(piece.as_str());
        if i == 7 {
            string.push('\n');
        }
    }
    string
}

fn text<T>(text: T, font: graphics::Font, scale: f32) -> Text
where
    T: Into<graphics::TextFragment>,
{
    let mut text = graphics::Text::new(text);
    text.set_font(font, graphics::Scale::uniform(scale));
    text
}
