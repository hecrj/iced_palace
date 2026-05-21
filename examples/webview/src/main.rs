use iced::widget::{column, text};
use iced::{Element, Font, Task};
use iced_palace::widget::webview;

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view).run()
}

struct Example {
    headers: webview::header::Map,
    cookies: webview::cookie::Jar,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(webview::Load),
    Ran(String),
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
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(load) => {
                dbg!(load);

                Task::batch([webview::run("webview-example", "document.cookie").map(Message::Ran)])
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
                .headers(&self.headers)
                .cookies(&self.cookies)
                .on_navigate(is_trusted)
                .on_load(Message::Loaded)
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
