use iced::widget::{center, column, toggler};
use iced::{Center, Element, Font};
use iced_palace::widget::dynamic_text;

fn main() -> iced::Result {
    iced::run(Example::update, Example::view)
}

#[derive(Default)]
struct Example {
    use_geometry: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleGeometry(bool),
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleGeometry(use_geometry) => {
                self.use_geometry = use_geometry;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        const ROY: &str = "I've seen things you people wouldn't believe.\n\
        Attack ships on fire off the shoulder of Orion.\n\
        I watched C-beams glitter in the dark near the Tannhäuser Gate.\n\
        All those moments will be lost in time, like tears in rain.";

        let toggle = toggler(self.use_geometry)
            .label("Use geometry")
            .on_toggle(Message::ToggleGeometry);

        center(
            column![
                dynamic_text(ROY)
                    .font(Font::MONOSPACE)
                    .center()
                    .width(500)
                    .line_height(1.5)
                    .vectorial(self.use_geometry),
                toggle
            ]
            .spacing(20)
            .align_x(Center),
        )
        .into()
    }
}
