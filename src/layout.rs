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
    // The size this Layout object will try to be. For a Rect, this is just the height and width
    // If this is None, then the object either has no opinion about its size, and uses the constraints given
    // to find its real size.
    fn preferred_size(&self) -> Option<(f32, f32)>;

    // Layout the child objects in this Layout. This Layout must not exceed the size of max_size.
    // This function returns the actual size of the laid out object.
    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32);

    // Set the position of this Layout in absolute coordnates
    fn set_position(&mut self, pos: Point2<f32>);
    // Set the relative position of this Layout.
    fn set_position_relative(&mut self, offset: Vector2<f32>);

    // None if the object has a fixed size prior to layout, Some if it is a dynamic size
    fn flex_factor(&self) -> Option<f32> {
        None
    }

    // The computed bounding box. This will panic if no bounding box exists.
    fn bounding_box(&self) -> Rect;
}
pub struct VStack<'a> {
    pub pos: Point2<f32>,
    pub children: &'a mut [&'a mut dyn Layout],
    // The minimum dimensions of the VStack. If this is not given, then the
    // VStack will try to be as small as possible.
    pub min_dimensions: (Option<f32>, Option<f32>),
}

impl<'a> Layout for VStack<'a> {
    fn preferred_size(&self) -> Option<(f32, f32)> {
        let (mut size_x, mut size_y) = self.min_size();

        if let Some(min_width) = self.min_dimensions.0 {
            size_x = size_x.max(min_width);
        }

        if let Some(min_height) = self.min_dimensions.1 {
            size_y = size_y.max(min_height);
        }

        Some((size_x, size_y))
    }

    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        let mut rel_y = 0.0;

        let mut flex_sum = 0.0;
        for child in self.children.iter() {
            flex_sum += child.flex_factor().unwrap_or(0.0);
        }

        // the amount of height each flexbox must share
        let size = self.preferred_size().unwrap();
        let remaining_height = size.1 - self.min_size().1;

        for (i, child) in self.children.iter_mut().enumerate() {
            // Now for positioning.
            // Just stack the children on top of each other with no padding.
            let max_child_height = match child.preferred_size() {
                None => {
                    remaining_height
                        * (child
                            .flex_factor()
                            .expect("Unsize children need to have a flex factor")
                            / flex_sum)
                }
                Some((_, height)) => height,
            };
            // Compute this child's height and width
            let (computed_width, computed_height) = child.layout((max_size.0, max_child_height));

            // Center the child vertically within the VStack.
            let centered_rect = Rect::new(0.0, rel_y, computed_width, computed_height);
            let centered_rect = center_vert(from_dims(size), centered_rect);
            // println!("ref_y: {:?}, child_re", rel_y);

            child.set_position(centered_rect.point());

            rel_y += computed_height;
        }

        debug_assert!(
            size.1 - rel_y < 1.0,
            format!("VStack height very different! {} {}", size.1, rel_y)
        );
        size
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

    fn bounding_box(&self) -> Rect {
        Rect::new(
            self.pos.x,
            self.pos.y,
            self.preferred_size().unwrap().0,
            self.preferred_size().unwrap().1,
        )
    }
}

impl<'a> VStack<'a> {
    // Return the minimum size this VStack can be in order to fit all the
    // children's preferred sizes. Children with no prefered size are assumed to be
    // zero size.
    fn min_size(&self) -> (f32, f32) {
        let mut size = (0.0f32, 0.0f32);
        for child in self.children.iter() {
            if let Some((child_width, child_height)) = child.preferred_size() {
                size.0 = size.0.max(child_width);
                size.1 += child_height;
            }
        }
        size
    }
}
impl<'a> HStack<'a> {
    // Return the minimum size this HStack can be in order to fit all the
    // children's preferred sizes. Children with no prefered size are assumed to be
    // zero size.
    fn min_size(&self) -> (f32, f32) {
        let mut size = (0.0f32, 0.0f32);
        for child in self.children.iter() {
            if let Some((child_width, child_height)) = child.preferred_size() {
                size.0 += child_width;
                size.1 = size.1.max(child_height);
            }
        }
        size
    }
}
pub struct HStack<'a> {
    pub pos: Point2<f32>,
    pub children: &'a mut [&'a mut dyn Layout],
    pub min_dimensions: (Option<f32>, Option<f32>),
}

impl<'a> Layout for HStack<'a> {
    fn preferred_size(&self) -> Option<(f32, f32)> {
        let (mut size_x, mut size_y) = self.min_size();

        if let Some(min_width) = self.min_dimensions.0 {
            size_x = size_x.max(min_width);
        }

        if let Some(min_height) = self.min_dimensions.1 {
            size_y = size_y.max(min_height);
        }

        Some((size_x, size_y))
    }

    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        let mut rel_x = 0.0;

        let mut flex_sum = 0.0;
        for child in self.children.iter() {
            flex_sum += child.flex_factor().unwrap_or(0.0);
        }

        let size = self.preferred_size().unwrap();
        // the amount of width each flexbox must share
        let remaining_width = size.0 - self.min_size().0;

        for (i, child) in self.children.iter_mut().enumerate() {
            // Now for positioning.
            // Just stack the children on top of each other with no padding.
            let max_child_width = match child.preferred_size() {
                None => {
                    remaining_width
                        * (child
                            .flex_factor()
                            .expect("Unsize children need to have a flex factor")
                            / flex_sum)
                }
                Some((width, _)) => width,
            };
            // Compute this child's height and width
            let (computed_width, computed_height) = child.layout((max_child_width, max_size.1));

            // Center the child vertically within the HStack.
            let centered_rect = Rect::new(rel_x, 0.0, computed_width, computed_height);
            let centered_rect = center_horiz(from_dims(size), centered_rect);

            child.set_position(centered_rect.point());

            rel_x += computed_width;
        }

        debug_assert!(
            size.0 - rel_x < 1.0,
            format!("HStack width very different! {} {}", size.0, rel_x)
        );
        size
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

    fn bounding_box(&self) -> Rect {
        Rect::new(
            self.pos.x,
            self.pos.y,
            self.preferred_size().unwrap().0,
            self.preferred_size().unwrap().1,
        )
    }
}

pub struct FlexBox {
    pub bounding_box: Option<Rect>,
    pub flex_factor: f32,
}

impl FlexBox {
    pub fn new(flex_factor: f32) -> FlexBox {
        FlexBox {
            bounding_box: None,
            flex_factor,
        }
    }
}

impl<'a> Layout for FlexBox {
    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        self.bounding_box = Some(from_dims(max_size));
        max_size
    }

    fn flex_factor(&self) -> Option<f32> {
        Some(self.flex_factor)
    }

    fn bounding_box(&self) -> Rect {
        self.bounding_box.unwrap()
    }

    fn preferred_size(&self) -> Option<(f32, f32)> {
        None
    }

    // TODO: this doens't need to be unwrap, let the position be seperate from the data
    fn set_position(&mut self, pos: Point2<f32>) {
        self.bounding_box.as_mut().unwrap().move_to(pos)
    }

    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        self.bounding_box.as_mut().unwrap().translate(offset)
    }
}

impl<'a> Layout for Button {
    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        self.hitbox.layout(max_size)
    }

    fn bounding_box(&self) -> Rect {
        self.hitbox
    }

    fn preferred_size(&self) -> Option<(f32, f32)> {
        self.hitbox.preferred_size()
    }

    fn set_position(&mut self, pos: Point2<f32>) {
        self.hitbox.move_to(pos);
    }

    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        self.hitbox.translate(offset);
    }
}

impl<'a> Layout for Rect {
    fn preferred_size(&self) -> Option<(f32, f32)> {
        Some((self.w, self.h))
    }

    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        (self.w, self.h)
    }

    fn bounding_box(&self) -> Rect {
        *self
    }

    fn set_position(&mut self, pos: Point2<f32>) {
        self.move_to(pos);
    }

    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        self.translate(offset);
    }

    fn flex_factor(&self) -> Option<f32> {
        None
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
