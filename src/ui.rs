use ggez::event::MouseButton;
use ggez::graphics::{self, DrawParam, Rect, Text};
use ggez::input;
use ggez::mint;
use ggez::{Context, GameResult};

use crate::color;
use crate::layout;
use crate::rect::{center_inside, from_dims, get_dims};

#[derive(Debug, Clone)]
pub struct Button {
    pub hitbox: Rect,
    state: ButtonState,
    pub text: TextBox,
}

impl Button {
    /// Return a button whose size is at least large enough to fit both min_hitbox
    /// and the text. If the text would be larger than min_hitbox, it is centered on top of
    /// min_hitbox.
    pub fn fit_to_text(ctx: &mut Context, min_dims: (f32, f32), text: Text) -> Button {
        let (w, h) = text.dimensions(ctx);
        let text_rect = from_dims((w as f32, h as f32));
        let min_hitbox = from_dims(min_dims);
        let hitbox = text_rect.combine_with(min_hitbox);

        Button {
            hitbox,
            state: ButtonState::Idle,
            text: TextBox {
                bounding_box: text_rect,
                text,
            },
        }
    }

    pub fn pressed(&self, mouse_pos: mint::Point2<f32>) -> bool {
        self.state == ButtonState::Pressed && self.hitbox.contains(mouse_pos)
    }

    pub fn upd8(&mut self, ctx: &mut Context) {
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

    pub fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        use ButtonState::*;
        let outer_color = color::WHITE;
        let inner_color = match self.state {
            Idle => graphics::Color::from_rgb_u32(0x13ff00),
            Hover => graphics::Color::from_rgb_u32(0x0ebf00),
            Pressed => graphics::Color::from_rgb_u32(0x0c9f00),
        };

        self.draw_with_color(ctx, outer_color, inner_color)
    }

    fn draw_with_color(
        &self,
        ctx: &mut Context,
        outer_color: graphics::Color,
        inner_color: graphics::Color,
    ) -> GameResult<()> {
        let fill: graphics::DrawMode = graphics::DrawMode::fill();
        let stroke_width = 3.0;
        let stroke: graphics::DrawMode = graphics::DrawMode::stroke(stroke_width);

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

        self.text.draw_with_color(ctx, color::BLACK)?;
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum ButtonState {
    Idle,
    Hover,
    Pressed,
}

#[derive(Debug)]
pub struct Selector {
    pub buttons: Vec<Button>,
    pub selected: usize,
}

impl Selector {
    pub fn new(buttons: Vec<Button>) -> Selector {
        Selector {
            buttons,
            selected: 0,
        }
    }

    pub fn upd8(&mut self, ctx: &mut Context) {
        for (i, button) in &mut self.buttons.iter_mut().enumerate() {
            button.upd8(ctx);
            if button.state == ButtonState::Pressed {
                self.selected = i;
            }
        }
    }

    pub fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        for (i, button) in self.buttons.iter().enumerate() {
            use ButtonState::*;
            let outer_color = if i == self.selected {
                graphics::Color::from_rgb_u32(0xE3F2FD)
            } else {
                color::WHITE
            };
            let inner_color = match (button.state, i) {
                (Pressed, _) => graphics::Color::from_rgb_u32(0x2979FF),
                (_, i) if i == self.selected => graphics::Color::from_rgb_u32(0x2962FF),
                (Hover, _) => graphics::Color::from_rgb_u32(0x448AFF),
                (Idle, _) => graphics::Color::from_rgb_u32(0x82B1FF),
            };

            button.draw_with_color(ctx, outer_color, inner_color)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TextBox {
    pub bounding_box: Rect,
    pub text: Text,
}

impl TextBox {
    pub fn new(dims: (f32, f32)) -> TextBox {
        TextBox {
            bounding_box: from_dims(dims),
            text: Text::default(),
        }
    }

    pub fn fit_to_text(ctx: &mut Context, text: Text) -> TextBox {
        let (w, h) = text.dimensions(ctx);
        let bounding_box = from_dims((w as f32, h as f32));
        TextBox { bounding_box, text }
    }

    fn draw_with_color(&self, ctx: &mut Context, color: graphics::Color) -> GameResult<()> {
        // DEBUG
        if layout::DEBUG_LAYOUT {
            let rect = &graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                self.bounding_box,
                color::TRANS_CYAN,
            )
            .unwrap();
            graphics::draw(ctx, rect, DrawParam::default())?;
        }

        let dims = (
            self.text.dimensions(ctx).0 as f32,
            self.text.dimensions(ctx).1 as f32,
        );
        let text_offset = center_inside(self.bounding_box, from_dims(dims));

        graphics::draw(ctx, &self.text, (text_offset.point(), color))?;

        draw_text_workaround(ctx);
        Ok(())
    }

    pub fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        self.draw_with_color(ctx, color::RED)
    }
}

/// Draw some text using the font, scale, and parameters specified.
pub fn draw_text<T, S>(
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
    graphics::draw(ctx, text, params)?;

    draw_text_workaround(ctx);
    Ok(())
}

/// This workaround is nessecary because after a draw call with text,
/// the DrawParam's dest is added to the next mesh draw.
/// This results in bizarre flicker problems where the next mesh draw is
/// displaced by the prior text draw's displacement. This fixes this issue
/// by resyncronizing the transform, suggesting it might be a memory barrier problem.
/// This issue started happened when I updated my Windows 10 laptop
/// so I guess a graphics API's behavior changed in some way.
pub fn draw_text_workaround(ctx: &mut Context) {
    ggez::graphics::apply_transformations(ctx)
        .expect("The Workaround Failed For Some Reason Oh God Oh Fuck");
}
