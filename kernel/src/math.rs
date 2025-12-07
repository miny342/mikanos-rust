use core::ops::Add;

#[derive(Debug, Clone)]
pub struct Vector2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2D<T> {
    pub fn new(x: T, y: T) -> Self {
        Vector2D { x, y }
    }
}

impl<T: Add<Output = T>> Add for Vector2D<T> {
    type Output = Vector2D<T>;
    fn add(self, rhs: Vector2D<T>) -> Self::Output {
        Vector2D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub pos: Vector2D<isize>,
    size: Vector2D<isize>,
}

impl Rectangle {
    pub fn new(pos: Vector2D<isize>, size: Vector2D<isize>) -> Self {
        assert!(size.x >= 0 && size.y >= 0);
        Rectangle { pos, size }
    }
    pub fn intersect(&self, other: &Rectangle) -> Option<Rectangle> {
        let x1 = self.pos.x.max(other.pos.x);
        let y1 = self.pos.y.max(other.pos.y);
        let x2 = (self.pos.x + self.size.x).min(other.pos.x + other.size.x);
        let y2 = (self.pos.y + self.size.y).min(other.pos.y + other.size.y);
        if x1 < x2 && y1 < y2 {
            Some(Rectangle::new(Vector2D::new(x1, y1), Vector2D::new(x2 - x1, y2 - y1)))
        } else {
            None
        }
    }
    pub fn set_size(&mut self, size: Vector2D<isize>) {
        assert!(size.x >= 0 && size.y >= 0);
        self.size = size;
    }
    pub fn size(&self) -> &Vector2D<isize> {
        &self.size
    }
    pub fn contain(&self, v: &Vector2D<isize>) -> bool {
        self.pos.x <= v.x && v.x < self.pos.x + self.size.x && self.pos.y <= v.y && v.y < self.pos.y + self.size.y
    }
}

mod tests {
    #[test_case]
    fn test_rectangle_intersection() {
        use super::{Rectangle, Vector2D};
        let r1 = Rectangle::new(Vector2D::new(0, 0), Vector2D::new(100, 100));
        let r2 = Rectangle::new(Vector2D::new(50, 50), Vector2D::new(100, 100));
        let r3 = Rectangle::new(Vector2D::new(200, 200), Vector2D::new(100, 100));
        let inter = r1.intersect(&r2).unwrap();
        assert_eq!(inter.pos.x, 50);
        assert_eq!(inter.pos.y, 50);
        assert_eq!(inter.size().x, 50);
        assert_eq!(inter.size().y, 50);
        assert!(r1.intersect(&r3).is_none());
    }
    #[test_case]
    fn test_rectangle_contain() {
        use super::{Rectangle, Vector2D};
        let r1 = Rectangle::new(Vector2D::new(0, 0), Vector2D::new(100, 100));
        assert!(r1.contain(&Vector2D::new(50, 50)));
        assert!(!r1.contain(&Vector2D::new(100, 0)));
        assert!(r1.contain(&Vector2D::new(0, 0)));
        assert!(!r1.contain(&Vector2D::new(0, 100)));
        assert!(!r1.contain(&Vector2D::new(-1, -1)));
    }
}