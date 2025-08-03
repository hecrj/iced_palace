use iced::widget::{center, center_x, column, container, row, text, toggler};
use iced::{Element, Font};
use iced_palace::widget::ellipsized_text;

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view).run()
}

struct Example {
    use_monospace: bool,
    wrap: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleMonospace(bool),
    ToggleWrap(bool),
}

impl Example {
    fn new() -> Self {
        Self {
            use_monospace: true,
            wrap: true,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleMonospace(use_monospace) => {
                self.use_monospace = use_monospace;
            }
            Message::ToggleWrap(wrap) => {
                self.wrap = wrap;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        const FROMM: &str = "\
            It is naively assumed that the fact that the majority of people \
            share certain ideas or feelings proves the validity of these ideas \
            and feelings. Nothing is further from the truth. Consensual validation \
            as such has no bearing on reason or mental health. Just as there is a \
            \"folie a deux\" there is a folie a millions. The fact that millions of \
            people share the same vices does not make these vices virtues, the fact \
            that they share so many errors does not make the errors to be truths, and \
            the fact that millions of people share the same forms of mental pathology \
            does not make these people sane.";

        let monospace_toggle = row![
            toggler(self.use_monospace)
                .label("Monospace")
                .on_toggle(Message::ToggleMonospace),
            toggler(self.wrap)
                .label("Wrap")
                .on_toggle(Message::ToggleWrap)
        ]
        .spacing(20);

        column![
            center(
                ellipsized_text(FROMM)
                    .font(if self.use_monospace {
                        Font::MONOSPACE
                    } else {
                        Font::DEFAULT
                    })
                    .line_height(1.5)
                    .size(20)
                    .wrapping(if self.wrap {
                        text::Wrapping::Word
                    } else {
                        text::Wrapping::None
                    })
            )
            .padding(10),
            center_x(monospace_toggle)
                .padding(10)
                .style(container::dark),
        ]
        .into()
    }
}
