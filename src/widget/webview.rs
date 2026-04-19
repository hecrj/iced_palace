use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::widget;
use crate::core::window;
use crate::core::{Element, Event, Layout, Length, Rectangle, Shell, Size, Widget};

#[cfg(target_os = "macos")]
use std::cell::Cell;

use std::cell::RefCell;
use std::rc::Rc;

pub struct Webview<'a, Message> {
    url: Url,
    width: Length,
    height: Length,
    headers: header::Map,
    on_navigate: fn(Url) -> bool,
    on_load: Option<Box<dyn Fn(Url) -> Message + 'a>>,
}

pub type Url = String;

pub mod header {
    pub use wry::http::HeaderMap as Map;
    pub use wry::http::HeaderName as Name;
    pub use wry::http::HeaderValue as Value;
}

impl<'a, Message> Webview<'a, Message> {
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            width: Length::Fill,
            height: Length::Fill,
            headers: header::Map::default(),
            on_navigate: |_| true,
            on_load: None,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn headers(mut self, headers: impl Into<header::Map>) -> Self {
        self.headers = headers.into();
        self
    }

    pub fn on_navigate(mut self, on_navigate: fn(Url) -> bool) -> Self {
        self.on_navigate = on_navigate;
        self
    }

    pub fn on_load(mut self, on_load: impl Fn(Url) -> Message + 'static) -> Self {
        self.on_load = Some(Box::new(on_load));
        self
    }
}

#[allow(clippy::large_enum_variant)]
enum State {
    New,
    Ready {
        webview: wry::WebView,
        url: Url,
        headers: header::Map,
        bounds: Rectangle,
        loads: Rc<RefCell<Vec<String>>>,
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
                    url,
                    headers,
                    ..
                } if url == &self.url && headers == &self.headers => {
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

                    let mut webview = wry::WebViewBuilder::new()
                        .with_url(&self.url)
                        .with_headers(self.headers.clone())
                        .with_navigation_handler(self.on_navigate)
                        .with_bounds(into_rect(bounds));

                    if self.on_load.is_some() {
                        let loads = loads.clone();
                        let ticker = shell.ticker().clone();

                        webview = webview.with_on_page_load_handler(move |event, url| {
                            let wry::PageLoadEvent::Finished = event else {
                                return;
                            };

                            loads.borrow_mut().push(url);
                            ticker.tick();
                        });
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

                    *state = State::Ready {
                        webview,
                        url: self.url.clone(),
                        headers: self.headers.clone(),
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
                    _ => mouse::Interaction::None,
                };
            }
        }

        if let Event::Tick = event {
            let state = tree.state.downcast_mut::<State>();

            let State::Ready { loads, .. } = state else {
                return;
            };

            if let Some(on_load) = &self.on_load {
                for url in loads.borrow_mut().drain(..) {
                    shell.publish(on_load(url));
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

#[cfg(target_os = "macos")]
const CURSOR_TRACKING: &str = r#"
function getEffectiveCursor(el) {
    let current = el;

    while (current) {
        // Explicit link detection
        if (current.closest && current.closest("a[href]")) {
            return "pointer";
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
