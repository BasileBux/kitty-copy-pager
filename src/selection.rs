use std::cmp::PartialEq;

#[derive(Clone)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}
impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Vec2 { x, y }
    }
}

impl<T: PartialEq> PartialEq for Vec2<T> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[derive(Clone)]
pub enum SelectedEnd {
    Start,
    End,
}

#[derive(Clone)]
pub struct Selection {
    pub start: Vec2<usize>,
    pub end: Vec2<usize>,
    pub sel_end: SelectedEnd,
}

impl Selection {
    pub fn new(start: Vec2<usize>, end: Vec2<usize>) -> Self {
        Selection {
            start,
            end,
            sel_end: SelectedEnd::End,
        }
    }

    pub fn with_coords(start_x: usize, start_y: usize, end_x: usize, end_y: usize) -> Self {
        Selection {
            start: Vec2::new(start_x, start_y),
            end: Vec2::new(end_x, end_y),
            sel_end: SelectedEnd::End,
        }
    }

    pub fn swap_ends_to(&mut self, to_x: usize, to_y: usize) {
        match self.sel_end {
            SelectedEnd::Start => {
                self.start = self.end.clone();
                self.end = Vec2::new(to_x, to_y);
            }
            SelectedEnd::End => {
                self.end = self.start.clone();
                self.start = Vec2::new(to_x, to_y);
            }
        }
    }
}
