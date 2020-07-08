use ggez::graphics::mint::{Point2, Vector2};
use ggez::graphics::Rect;

use crate::rect::*;
use crate::screen::*;

/// Trait which indicates that an object can be laid out onscreen in some fashion.
/// This based on the Flutter algorithm for laying out objects.
/// In the Flutter model, each Layout object is given a certain size constraint
/// it must fit into. Layout objects may contain children Layouts.
/// The parent's job when `layout` is called is to ask each child's `preferred_size`,
/// and using that information, place the children appropriately. The `layout`
/// function also has the parent return its actual size. (Note that the actual size)
/// and prefered size may different depending on constraints. Some objects also
/// do not have a prefered size. For example, flex objects do not care how large
/// they are, and instead are dynamically sized by the parent based on how much
/// space remaining there is.
/// Any Layout object assumes that it has both a position (that it does not control)
/// and a size (that it has some control over). Implementors of Layout should note that
/// prior to a `layout` call, objects are not positioned anywhere, and afterwards
/// are positioned such that (0, 0) is treated as **the parent's upper left corner**.
/// This means you MUST call `set_position_relative` after laying out, even if the
/// offset is zero.
/// Note that this implementation is somewhat naive, and assumes that
/// Layout objects can always fit their children. This is probably fine though.
pub trait Layout {
    /// The size this Layout object will try to be. For a Rect, this is just the
    ///- height and width. If this returns None, then the object has no opinion
    // about its size, and should be sized by the parent instead.
    fn preferred_size(&self) -> Option<(f32, f32)>;

    /// Layout the child objects in this Layout, positioning the children appropriately
    /// This Layout must not exceed the size of max_size.
    /// This function returns the actual size of the laid out object. This actual
    /// size may be different from the prefered size due to constraints.
    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32);

    /// Set the position of this Layout in absolute coordinates, where (0, 0)
    /// is the upper left corner of the parent containing this Layout object
    fn set_position(&mut self, pos: Point2<f32>);

    /// Set the relative position of this Layout. Note that implementors should
    /// remember that the coordinates of the child are relative to itself own
    /// upper left corner, so you should typically write code that looks like this
    ///
    /// ```ignore
    /// fn set_position_relative(&mut self, offset: Vector2<f32>) {
    ///     // set own position first
    ///     self.my_bounding_box.set_position_relative(offset);
    ///     // set child's position with an offset effectively equal to offset + own size
    ///     self.my_child.set_position_relative(self.bounding_box().point())
    /// }
    /// ```
    fn set_position_relative(&mut self, offset: Vector2<f32>);

    /// This function returns None if the object is rigid, and Some if this object
    /// is a flex/dynamically sized object. If this returns None, then `preferred_size`
    /// **must return Some**
    fn flex_factor(&self) -> Option<f32> {
        None
    }

    /// The object's actual bounding box. Note that this should panic if no bounding
    /// box exists at the moment (ex: it is a `FlexBox` that hasn't had `layout`
    /// called on it, and therefore has no computed size).
    fn bounding_box(&self) -> Rect;
}

/// A container that lays out its children vertically stacked on each other.
/// This container does not have any padding on its children.
pub struct VStack<'a> {
    /// The upper left corner of the VStack's bounding box
    pub pos: Point2<f32>,
    /// The children of this VStack.
    pub children: &'a mut [&'a mut dyn Layout],
    /// The minimum dimensions of the VStack. If this is None, then the VStack
    /// will try to be as small as possible, and will assume a size of zero
    /// for any flex objects.
    pub min_dimensions: (Option<f32>, Option<f32>),
}

impl<'a> Layout for VStack<'a> {
    /// The VStack's preferred size is the smallest rectangle containing all of
    /// its rigid children and the minimum dimensions (if they exist).
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
        // The relative y-coordinate of this VStack, measured from the top of
        // the VStack
        let mut rel_y = 0.0;

        // Compute the amount of vertical space the flex objects will need to share.
        // This value is equal to the preferred height minus the sum of heights of
        // the rigid children. Note that this means that if there is no minimum
        // height, then flex objects get no space.
        let mut flex_sum = 0.0;
        for child in self.children.iter() {
            flex_sum += child.flex_factor().unwrap_or(0.0);
        }

        let size = self.preferred_size().unwrap();
        let remaining_height = size.1 - self.min_size().1;

        // Now for positioning.
        for child in self.children.iter_mut() {
            // This is the max height the child may be. If it's rigid, then we
            // just let it be its prefered height. Otherwise, we give it its
            // share of the remaining_height
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
            // Actually layout the child.
            let (computed_width, computed_height) = child.layout((max_size.0, max_child_height));

            // We will position the child centered vertically in the VStack, and
            // place it immediately after the previous object.
            let centered_rect = Rect::new(0.0, rel_y, computed_width, computed_height);
            let centered_rect = center_vert(from_dims(size), centered_rect);
            child.set_position(centered_rect.point());

            rel_y += computed_height;
        }

        // Hopefully, the actual size and prefered height of this object are the same
        debug_assert!(
            (size.1 - rel_y).abs() < 1.0,
            format!(
                "VStack height very different! preferred: {} real: {}",
                size.1, rel_y
            )
        );

        // Note that the max width and real width need not be similar, as we always
        // pick the smallest width possible, which might be much smaller than
        // the max (ex: we have many thin objects, but are given a very large
        // max width)

        if size.1 > max_size.1 {
            println!(
                "WARNING: Overfull VStack! max_size: {:?} actual_size: {:?}",
                max_size, size
            );
        }

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
    /// The minimum size this VStack can be if all flex objects were given
    /// zero size.
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

        for child in self.children.iter_mut() {
            // Now for positioning.
            // Just stack the children on top of each other with no padding. We
            // will not handle cases where the children overflow its parent

            // The child is allowed to be either its preferred size or is a portion
            // of the remaining_width based on its flex factor
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

            // Compute this child's height and width and have it also layout
            let (computed_width, computed_height) = child.layout((max_child_width, max_size.1));

            // Center the child vertically within the HStack.
            let centered_rect = Rect::new(rel_x, 0.0, computed_width, computed_height);
            let centered_rect = center_horiz(from_dims(size), centered_rect);
            // Note that we are setting this where (0, 0) = top right corner of
            // this HStack. We will later have to set the relative position
            // so that things correctly draw on screen, otherwise it will look wrong.
            child.set_position(centered_rect.point());

            // The relative x position from the top right corner of the HStack
            rel_x += computed_width;
        }
        // If everything is correct, then the HStack should always be the same size
        // as the prefered size.
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

/// A FlexBox is a flex object whose size is determined by the parent.
/// A higher flex factor indicates that it should have more space, relative
/// to other flex objects. This object does not have a bounding_box until
/// `layout` is called
#[derive(Debug, Clone)]
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

impl<'a> Layout for Button {
    fn layout(&mut self, max_size: (f32, f32)) -> (f32, f32) {
        let hitbox_size = self.hitbox.layout(max_size);
        self.text.layout(max_size);

        let centered_rect =
            center_inside(get_dims(self.hitbox), get_dims(self.text.bounding_box()));
        self.text.set_position(centered_rect.point());

        hitbox_size
    }

    fn bounding_box(&self) -> Rect {
        self.hitbox
    }

    fn preferred_size(&self) -> Option<(f32, f32)> {
        self.hitbox.preferred_size()
    }

    fn set_position(&mut self, pos: Point2<f32>) {
        self.hitbox.set_position(pos);
    }

    fn set_position_relative(&mut self, offset: Vector2<f32>) {
        self.hitbox.set_position_relative(offset);
        let child_offset = Vector2::from(self.hitbox.point());
        self.text.set_position_relative(child_offset);
    }
}

impl<'a> Layout for Rect {
    fn preferred_size(&self) -> Option<(f32, f32)> {
        Some((self.w, self.h))
    }

    fn layout(&mut self, _max_size: (f32, f32)) -> (f32, f32) {
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
