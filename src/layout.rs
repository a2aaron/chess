use ggez::graphics::mint::{Point2, Vector2};
use ggez::graphics::Rect;

use crate::rect::*;
use crate::screen::*;

/// Based on the Flutter algorithm for laying out objects.
/// The idea is that each Layout object gets to know how large it is, but not where it is located
/// Each Layout may have a number of children Layouts. The parent Layout, using the child's sizes
/// decides how to place each of its children.
/// Note that this implementation is somewhat naive, and assumes that
/// Layout objects can always fit their children. This is probably fine though.
pub trait Layout {
    // The size of this Layout object. For a Rect, this is just the height and width
    fn size(&self) -> (f32, f32);
    // Layout the child objects in this Layout. This Layout must not exceed the size of max_size
    fn layout(&mut self, max_size: (f32, f32));
    // Set the position of this Layout in absolute coordnates
    fn set_position(&mut self, pos: Point2<f32>);
    // Set the relative position of this Layout.
    fn set_position_relative(&mut self, offset: Vector2<f32>);
}
pub struct VStack<'a> {
    pub pos: Point2<f32>,
    pub children: &'a mut [&'a mut dyn Layout],
    pub min_width: f32, // The minimum width that this VStack is allowed to be. Use 0.0 for no minimum
}

impl<'a> Layout for VStack<'a> {
    fn size(&self) -> (f32, f32) {
        let (mut size_x, mut size_y) = (self.min_width, 0.0f32);
        for child in self.children.iter() {
            let child_size = child.size();
            size_x = size_x.max(child_size.0);
            size_y += child_size.1;
        }
        (size_x, size_y)
    }

    fn layout(&mut self, max_size: (f32, f32)) {
        let mut rel_y = 0.0;
        let size = self.size();
        for (i, child) in self.children.iter_mut().enumerate() {
            // Now for positioning.
            // Just stack the children on top of each other with no padding.
            // Center the child vertically within the VStack.
            let child_rect = Rect::new(0.0, rel_y, child.size().0, child.size().1);
            let child_rect = center_vert(from_dims(size), child_rect);
            child.set_position(child_rect.point());
            child.layout((max_size.0, max_size.1 - rel_y));

            rel_y += child.size().1;
        }
    }

    fn set_position(&mut self, pos: Point2<f32>) {
        self.pos = pos;
    }

    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        // First, move ourselves to the correct position
        self.pos = Point2 {
            x: offset.x + self.pos.x,
            y: offset.y + self.pos.y,
        };
        for child in self.children.iter_mut() {
            // We stored the child position in relative coordinates, but now we
            // need to go to absolute coordinates, so we move by however much
            // bounding_box is offset.
            let child_offset = Vector2::from(self.pos);
            child.set_position_relative(child_offset);
        }
    }
}

impl<'a> VStack<'a> {
    pub fn bounding_box(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.size().0, self.size().1)
    }
}
impl<'a> HStack<'a> {
    pub fn bounding_box(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.size().0, self.size().1)
    }
}

pub struct HStack<'a> {
    pub pos: Point2<f32>,
    pub children: &'a mut [&'a mut dyn Layout],
}

impl<'a> Layout for HStack<'a> {
    fn size(&self) -> (f32, f32) {
        let (mut size_x, mut size_y) = (0.0f32, 0.0f32);
        for child in self.children.iter() {
            let child_size = child.size();
            size_x += child_size.0;
            size_y = size_y.max(child_size.1);
        }
        (size_x, size_y)
    }

    fn layout(&mut self, max_size: (f32, f32)) {
        let mut rel_x = 0.0;
        let size = self.size();
        for (i, child) in self.children.iter_mut().enumerate() {
            // Now for positioning.
            // Just stack the children on top of each other with no padding.
            // Center the child horizontally within the HStack.
            let child_rect = Rect::new(rel_x, 0.0, child.size().0, child.size().1);
            let child_rect = center_horiz(from_dims(size), child_rect);
            child.set_position(child_rect.point());
            child.layout((max_size.0 - rel_x, max_size.1));

            rel_x += child.size().0;
        }
    }

    fn set_position(&mut self, pos: Point2<f32>) {
        self.pos = pos;
    }

    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        // First, move ourselves to the correct position
        self.pos = Point2 {
            x: offset.x + self.pos.x,
            y: offset.y + self.pos.y,
        };
        for child in self.children.iter_mut() {
            // We stored the child position in relative coordinates, but now we
            // need to go to absolute coordinates, so we move by however much
            // bounding_box is offset.
            let child_offset = Vector2::from(self.pos);
            child.set_position_relative(child_offset);
        }
    }
}

impl<'a> Layout for Rect {
    fn size(&self) -> (f32, f32) {
        (self.w, self.h)
    }

    fn layout(&mut self, max_size: (f32, f32)) {}

    fn set_position(&mut self, offset: Point2<f32>) {
        self.move_to(offset);
    }
    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        self.translate(offset);
    }
}

impl<'a> Layout for Button {
    fn size(&self) -> (f32, f32) {
        self.hitbox.size()
    }

    fn layout(&mut self, max_size: (f32, f32)) {
        self.hitbox.layout(max_size);
    }

    fn set_position(&mut self, offset: Point2<f32>) {
        self.hitbox.move_to(offset);
    }
    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        self.hitbox.translate(offset);
    }
}

fn divide_vert(num_rects: u32, bounding_box: Rect) -> Vec<Rect> {
    let offset_x = bounding_box.x;
    let offset_y = bounding_box.y;
    let width = bounding_box.w;
    let height = bounding_box.h / num_rects as f32;
    let mut rects = vec![];
    for i in 0..num_rects {
        rects.push(Rect::new(
            offset_x,
            i as f32 * height + offset_y,
            width,
            height,
        ));
    }
    rects
}
