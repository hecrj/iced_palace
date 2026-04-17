use iced::widget::{column, text};
use iced::{Element, Font};
use iced_palace::widget::webview;

fn main() -> iced::Result {
    iced::run(Example::update, Example::view)
}

#[derive(Default)]
struct Example;

#[derive(Debug)]
enum Message {}

impl Example {
    fn update(&mut self, message: Message) {
        match message {}
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("webview widget!").font(Font::MONOSPACE),
            webview("https://google.com/")
        ]
        .spacing(20)
        .padding(20)
        .into()
    }
}
