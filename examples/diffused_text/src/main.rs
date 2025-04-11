use iced::time::milliseconds;
use iced::widget::{center, column};
use iced::{Center, Element, Font};

use iced_palace::widget::diffused_text;

fn main() -> iced::Result {
    iced::run(Example::update, Example::view)
}

struct Example {
    text: String,
}

#[derive(Debug)]
enum Message {}

impl Example {
    fn update(&mut self, message: Message) {
        match message {}
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![
                diffused_text("Diffused Text")
                    .size(20)
                    .font(Font::MONOSPACE),
                diffused_text(&self.text)
                    .duration(milliseconds(20) * self.text.len() as u32)
                    .font(Font::MONOSPACE)
                    .width(400)
                    .center()
            ]
            .align_x(Center)
            .spacing(20),
        )
        .into()
    }
}

impl Default for Example {
    fn default() -> Self {
        Self {
            text: "What is real? How do you define 'real'? If you're talking about \
            what you can feel, what you can smell, what you can taste and see, \
            then 'real' is simply electrical signals interpreted by your brain."
                .to_owned(),
        }
    }
}
