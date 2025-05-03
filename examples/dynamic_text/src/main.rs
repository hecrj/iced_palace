use iced::keyboard;
use iced::widget::{center_x, column, container, row, toggler};
use iced::{Element, Fill, Font, Subscription};
use iced_palace::widget::dynamic_text;

fn main() -> iced::Result {
    iced::application(Example::default, Example::update, Example::view)
        .subscription(Example::subscription)
        .run()
}

#[derive(Default)]
struct Example {
    use_geometry: bool,
    use_monospace: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleGeometry(bool),
    ToggleMonospace(bool),
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleGeometry(use_geometry) => {
                self.use_geometry = use_geometry;
            }
            Message::ToggleMonospace(use_monospace) => {
                self.use_monospace = use_monospace;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        const ROY: &str = "I've seen things you people wouldn't believe.\n\
        Attack ships on fire off the shoulder of Orion.\n\
        I watched C-beams glitter in the dark near the Tannhäuser Gate.\n\
        All those moments will be lost in time, like tears in rain.";

        let geometry_toggle = toggler(self.use_geometry)
            .label("Geometry")
            .on_toggle(Message::ToggleGeometry);

        let monospace_toggle = toggler(self.use_monospace)
            .label("Monospace")
            .on_toggle(Message::ToggleMonospace);

        column![
            dynamic_text(ROY)
                .font(if self.use_monospace {
                    Font::MONOSPACE
                } else {
                    Font::DEFAULT
                })
                .width(Fill)
                .height(Fill)
                .center()
                .line_height(1.5)
                .vectorial(self.use_geometry),
            center_x(row![geometry_toggle, monospace_toggle].spacing(30))
                .padding(10)
                .style(container::dark),
        ]
        .spacing(10)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, _modifiers| match key {
            keyboard::Key::Named(keyboard::key::Named::Space) => Some(Message::ToggleGeometry),
            _ => None,
        })
        .with(self.use_geometry)
        .map(|(use_geometry, f)| f(!use_geometry))
    }
}
