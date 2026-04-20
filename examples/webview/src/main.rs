use iced::widget::{column, text};
use iced::{Element, Font, Task, never};
use iced_palace::widget::webview;

fn main() -> iced::Result {
    iced::run(Example::update, Example::view)
}

#[derive(Default)]
struct Example;

#[derive(Debug, Clone)]
enum Message {
    Loaded(webview::Load),
    Ran(String),
}

impl Example {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(load) => {
                dbg!(load);

                webview::run("webview-example", "document.documentElement.outerHTML").map(never)
            }
            Message::Ran(result) => {
                dbg!(result);

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("webview widget!").font(Font::MONOSPACE),
            webview("https://iced.rs")
                .id("webview-example")
                .on_navigate(is_trusted)
                .on_load(Message::Loaded)
                .on_run(Message::Ran)
        ]
        .spacing(20)
        .padding(20)
        .into()
    }
}

fn is_trusted(url: webview::Url) -> bool {
    url.domain()
        .is_some_and(|domain| domain == "iced.rs" || domain.ends_with(".iced.rs"))
}
