use iced::widget::{center, center_x, column, container, toggler};
use iced::{Element, Font};
use iced_palace::widget::typewriter;

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view).run()
}

struct Example {
    use_monospace: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleMonospace(bool),
}

impl Example {
    fn new() -> Self {
        Self {
            use_monospace: true,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleMonospace(use_monospace) => {
                self.use_monospace = use_monospace;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        const JOI: &str = "Mere data makes a man.\n\
        A and C and T and G.\n\
        The alphabet of you.\n\
        All from four symbols.\n\
        I am only two: 1 and 0.\n
    — Joi, Blade Runner 2049";

        let monospace_toggle = toggler(self.use_monospace)
            .label("Monospace")
            .on_toggle(Message::ToggleMonospace);

        column![
            center(
                typewriter(JOI)
                    .font(if self.use_monospace {
                        Font::MONOSPACE
                    } else {
                        Font::DEFAULT
                    })
                    .line_height(1.5)
                    .very_slow()
            ),
            center_x(monospace_toggle)
                .padding(10)
                .style(container::dark),
        ]
        .spacing(10)
        .into()
    }
}
