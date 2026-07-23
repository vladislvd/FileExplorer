use eframe::egui::{CursorIcon, Response};

pub trait PointHandTrait{
    fn hand_cursor(self) -> Self;
}

impl PointHandTrait for Response {
    fn hand_cursor(self) -> Self {
        self.on_hover_cursor(CursorIcon::PointingHand)
    }
}

pub trait TextHandTrait{
    fn text_cursor(self) -> Self;
}

impl TextHandTrait for Response {
    fn text_cursor(self) -> Self {
        self.on_hover_cursor(CursorIcon::Text)
    }
}