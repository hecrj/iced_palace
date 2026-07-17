use iced::keyboard;
use iced::widget::{column, text};
use iced::{Element, Font, Subscription, Task};
use iced_palace::widget::webview;

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view)
        .subscription(Example::subscription)
        .scale_factor(Example::scale_factor)
        .run()
}

struct Example {
    headers: webview::header::Map,
    cookies: webview::cookie::Jar,
    scale_factor: f32,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(webview::Load),
    Errored(webview::Error),
    Ran(String),
    Keyboard(keyboard::Event),
}

impl Example {
    fn new() -> Self {
        let some_cookie = webview::Cookie::build(("some-cookie", "some-value"))
            .domain("iced.rs")
            .path("/")
            .build();

        let another_cookie = webview::Cookie::build(("another-cookie", "another-value"))
            .domain("iced.rs")
            .path("/")
            .build();

        Self {
            headers: webview::header::Map::new(),
            cookies: vec![some_cookie, another_cookie],
            scale_factor: 1.0,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(load) => {
                dbg!(load);

                Task::batch([webview::run("webview-example", "document.cookie").map(Message::Ran)])
            }
            Message::Errored(error) => {
                dbg!(error);

                Task::none()
            }
            Message::Ran(result) => {
                dbg!(result);

                Task::none()
            }
            Message::Keyboard(keyboard::Event::KeyPressed {
                modified_key,
                modifiers,
                ..
            }) => {
                match modified_key.as_ref() {
                    keyboard::Key::Character("=") if modifiers.command() => {
                        self.scale_factor += 0.25;
                    }
                    keyboard::Key::Character("-") if modifiers.command() => {
                        self.scale_factor -= 0.25;
                        self.scale_factor = self.scale_factor.max(0.5);
                    }
                    _ => {}
                }

                Task::none()
            }
            Message::Keyboard(_) => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("webview widget!").font(Font::MONOSPACE),
            webview("https://iced.rs")
                .id("webview-example")
                .headers(&self.headers)
                .cookies(&self.cookies)
                .on_navigate(is_trusted)
                .on_load(Message::Loaded)
                .on_error(Message::Errored)
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().map(Message::Keyboard)
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}

fn is_trusted(url: webview::Url) -> bool {
    url.domain()
        .is_some_and(|domain| domain == "iced.rs" || domain.ends_with(".iced.rs"))
}
