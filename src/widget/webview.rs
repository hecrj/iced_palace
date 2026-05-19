use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::widget;
use crate::core::window;
use crate::core::{Element, Event, Layout, Length, Rectangle, Shell, Size, Widget};

use iced_runtime::Task;
use iced_runtime::futures::futures::channel::oneshot;
use iced_runtime::task;

use std::borrow::Cow;

#[cfg(target_os = "macos")]
use std::cell::Cell;

use std::cell::RefCell;
use std::rc::Rc;

pub use cookie::Cookie;
pub use url::Url;

pub struct Webview<'a, Message> {
    source: Source<'a>,
    width: Length,
    height: Length,
    headers: Cow<'a, header::Map>,
    cookies: Cow<'a, [Cookie]>,
    on_navigate: fn(Url) -> bool,
    on_load: Option<Box<dyn Fn(Load) -> Message + 'a>>,
    id: Option<widget::Id>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source<'a> {
    Url(Cow<'a, str>),
    Html(Cow<'a, str>),
}

impl Source<'_> {
    fn to_static(&self) -> Source<'static> {
        match self {
            Source::Url(url) => Source::Url(Cow::Owned(url.clone().into_owned())),
            Source::Html(html) => Source::Html(Cow::Owned(html.clone().into_owned())),
        }
    }
}

impl From<String> for Source<'_> {
    fn from(url: String) -> Self {
        Source::Url(url.into())
    }
}

impl<'a> From<&'a str> for Source<'a> {
    fn from(url: &'a str) -> Self {
        Source::Url(Cow::Borrowed(url))
    }
}

pub mod header {
    pub use wry::http::HeaderMap as Map;
    pub use wry::http::HeaderName as Name;
    pub use wry::http::HeaderValue as Value;

    pub use wry::http::header::COOKIE;
    pub use wry::http::header::ORIGIN;
    pub use wry::http::header::REFERER;
    pub use wry::http::header::REFERRER_POLICY;
    pub use wry::http::header::USER_AGENT;

    use std::borrow::Cow;

    pub trait IntoMap<'a> {
        fn into(self) -> Cow<'a, Map>;
    }

    impl<'a> IntoMap<'a> for Map {
        fn into(self) -> Cow<'a, Map> {
            Cow::Owned(self)
        }
    }

    impl<'a> IntoMap<'a> for &'a Map {
        fn into(self) -> Cow<'a, Map> {
            Cow::Borrowed(self)
        }
    }
}

pub mod cookie {
    pub use wry::cookie::CookieBuilder as Builder;
    pub use wry::cookie::SameSite;

    pub type Cookie = wry::cookie::Cookie<'static>;
    pub type Jar = Vec<Cookie>;

    use std::borrow::Cow;

    pub trait IntoJar<'a> {
        fn into(self) -> Cow<'a, [Cookie]>;
    }

    impl<'a> IntoJar<'a> for Jar {
        fn into(self) -> Cow<'a, [Cookie]> {
            Cow::Owned(self)
        }
    }

    impl<'a> IntoJar<'a> for &'a [Cookie] {
        fn into(self) -> Cow<'a, [Cookie]> {
            Cow::Borrowed(self)
        }
    }

    impl<'a> IntoJar<'a> for &'a Jar {
        fn into(self) -> Cow<'a, [Cookie]> {
            Cow::Borrowed(self)
        }
    }
}

impl<'a, Message> Webview<'a, Message> {
    pub fn new(source: impl Into<Source<'a>>) -> Self {
        Self {
            source: source.into(),
            width: Length::Fill,
            height: Length::Fill,
            headers: Cow::Owned(header::Map::default()),
            cookies: Cow::Owned(cookie::Jar::default()),
            on_navigate: |_| true,
            on_load: None,
            id: None,
        }
    }

    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn headers(mut self, headers: impl header::IntoMap<'a>) -> Self {
        self.headers = headers.into();
        self
    }

    pub fn cookies(mut self, cookies: impl cookie::IntoJar<'a>) -> Self {
        self.cookies = cookies.into();
        self
    }

    pub fn on_navigate(mut self, on_navigate: fn(Url) -> bool) -> Self {
        self.on_navigate = on_navigate;
        self
    }

    pub fn on_load(mut self, on_load: impl Fn(Load) -> Message + 'a) -> Self {
        self.on_load = Some(Box::new(on_load));
        self
    }
}

#[allow(clippy::large_enum_variant)]
enum State {
    New,
    Ready {
        webview: wry::WebView,
        source: Source<'static>,
        headers: header::Map,
        cookies: cookie::Jar,
        bounds: Rectangle,
        loads: Rc<RefCell<Vec<Load>>>,
        #[cfg(target_os = "macos")]
        cursor: Rc<Cell<Option<String>>>,
        #[cfg(target_os = "macos")]
        interaction: mouse::Interaction,
    },
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Webview<'a, Message>
where
    Message: 'a,
    Renderer: crate::core::Renderer,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::New)
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &iced_core::layout::Limits,
    ) -> layout::Node {
        let size = limits.width(self.width).height(self.height).resolve(
            self.width,
            self.height,
            Size::ZERO,
        );

        layout::Node::new(size)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            let state = tree.state.downcast_mut::<State>();

            match state {
                State::Ready {
                    webview,
                    bounds,
                    source,
                    headers,
                    cookies,
                    ..
                } if *source == self.source
                    && headers == self.headers.as_ref()
                    && cookies == self.cookies.as_ref() =>
                {
                    let new_bounds = layout.bounds();

                    if *bounds != new_bounds {
                        let _ = webview.set_bounds(into_rect(new_bounds));
                        *bounds = new_bounds;
                    }
                }
                _ => {
                    let bounds = layout.bounds();
                    let loads = Rc::new(RefCell::new(Vec::new()));
                    #[cfg(target_os = "macos")]
                    let cursor = Rc::new(Cell::new(None));

                    let webview = wry::WebViewBuilder::new()
                        .with_headers(self.headers.clone().into_owned())
                        .with_navigation_handler({
                            let on_navigate = self.on_navigate;

                            move |url| {
                                let Ok(url) = Url::parse(&url) else {
                                    return false;
                                };

                                on_navigate(url)
                            }
                        })
                        .with_bounds(into_rect(bounds));

                    let mut webview = match &self.source {
                        Source::Url(url) => webview.with_url(url.clone()),
                        Source::Html(html) => webview.with_html(html.clone()),
                    };

                    if self.on_load.is_some() {
                        let loads = loads.clone();
                        let waker = shell.waker().clone();

                        webview = webview.with_on_page_load_handler(move |event, url| {
                            let Ok(url) = Url::parse(&url) else {
                                return;
                            };

                            loads.borrow_mut().push(match event {
                                wry::PageLoadEvent::Started => Load::Started(url),
                                wry::PageLoadEvent::Finished => Load::Finished(url),
                            });

                            waker.wake();
                        });
                    }

                    if let Some(user_agent) = self
                        .headers
                        .get(header::USER_AGENT)
                        .map(header::Value::to_str)
                        .and_then(Result::ok)
                    {
                        webview = webview.with_user_agent(user_agent);
                    }

                    #[cfg(target_os = "macos")]
                    let webview = webview
                        .with_initialization_script(CURSOR_TRACKING)
                        .with_ipc_handler({
                            let cursor = cursor.clone();

                            move |request| {
                                let _ = cursor.replace(Some(request.body().to_owned()));
                            }
                        });

                    let webview = webview
                        .build_as_child(&shell.window())
                        .expect("start webview");

                    for cookie in self.cookies.iter() {
                        let _ = webview.set_cookie(cookie);
                    }

                    *state = State::Ready {
                        webview,
                        source: self.source.to_static(),
                        headers: self.headers.clone().into_owned(),
                        cookies: self.cookies.clone().into_owned(),
                        bounds,
                        loads,
                        #[cfg(target_os = "macos")]
                        cursor,
                        #[cfg(target_os = "macos")]
                        interaction: mouse::Interaction::None,
                    };
                }
            }
        }

        #[cfg(target_os = "macos")]
        if let Event::Mouse(_) = event {
            let state = tree.state.downcast_mut::<State>();

            let State::Ready {
                cursor,
                interaction,
                ..
            } = state
            else {
                return;
            };

            if let Some(cursor) = cursor.take() {
                *interaction = match cursor.as_str() {
                    "pointer" => mouse::Interaction::Pointer,
                    "ew-resize" => mouse::Interaction::ResizingHorizontally,
                    "text" => mouse::Interaction::Text,
                    _ => mouse::Interaction::None,
                };
            }
        }

        if let Event::Waken = event {
            let state = tree.state.downcast_mut::<State>();

            let State::Ready { loads, .. } = state else {
                return;
            };

            if let Some(on_load) = &self.on_load {
                for load in loads.borrow_mut().drain(..) {
                    shell.publish(on_load(load));
                }
            }
        }
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // This is a no-op.
        // `wry` handles drawing for us in a child window
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        #[cfg(target_os = "macos")]
        {
            if !_cursor.is_over(_layout.bounds()) {
                return mouse::Interaction::None;
            }

            let State::Ready { interaction, .. } = _tree.state.downcast_ref::<State>() else {
                return mouse::Interaction::None;
            };

            *interaction
        }

        #[cfg(not(target_os = "macos"))]
        mouse::Interaction::None
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.custom(
            self.id.as_ref(),
            layout.bounds(),
            tree.state.downcast_mut::<State>() as _,
        );
    }
}

pub fn run(target: impl Into<widget::Id>, javascript: impl Into<String>) -> Task<String> {
    struct Run {
        target: widget::Id,
        javascript: String,
        sender: Option<oneshot::Sender<String>>,
    }

    impl<T> widget::Operation<T> for Run {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation<T>)) {
            if self.sender.is_some() {
                operate(self);
            }
        }

        fn custom(
            &mut self,
            id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn std::any::Any,
        ) {
            use std::sync::Mutex;

            if id != Some(&self.target) {
                return;
            }

            let Some(State::Ready { webview, .. }) = state.downcast_ref::<State>() else {
                return;
            };

            let Some(sender) = self.sender.take() else {
                return;
            };

            let sender = Mutex::new(Some(sender));

            let _ = webview.evaluate_script_with_callback(&self.javascript, move |result| {
                let Some(sender) = sender.lock().ok().as_deref_mut().and_then(Option::take) else {
                    return;
                };

                let _ = sender.send(result);
            });
        }
    }

    let (sender, receiver) = oneshot::channel();

    let run = task::widget(Run {
        target: target.into(),
        javascript: javascript.into(),
        sender: Some(sender),
    });

    let receive = Task::future(async move {
        let Ok(result) = receiver.await else {
            return String::new();
        };

        result
    });

    Task::batch([run, receive])
}

impl<'a, Message, Theme, Renderer> From<Webview<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: crate::core::Renderer,
{
    fn from(webview: Webview<'a, Message>) -> Self {
        Element::new(webview)
    }
}

fn into_rect(bounds: Rectangle) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(
            f64::from(bounds.x),
            f64::from(bounds.y),
        )),
        size: wry::dpi::Size::Logical(wry::dpi::LogicalSize::new(
            f64::from(bounds.width),
            f64::from(bounds.height),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Load {
    Started(Url),
    Finished(Url),
}

#[cfg(target_os = "macos")]
const CURSOR_TRACKING: &str = r#"
function getEffectiveCursor(el) {
    let current = el;

    while (current) {
        // Explicit link detection
        if (current.closest && current.closest("a[href]")) {
            return "pointer";
        }

        if (current.closest && current.closest("input")) {
            return "text";
        }

        const style = getComputedStyle(current);
        if (style.cursor && style.cursor !== "auto") {
            return style.cursor;
        }

        current = current.parentElement;
    }

    return "default";
}

document.addEventListener("mouseover", (e) => {
    const cursor = getEffectiveCursor(e.target);
    window.ipc.postMessage(cursor);
});
"#;
