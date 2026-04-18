use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::widget;
use crate::core::window;
use crate::core::{Element, Event, Layout, Length, Rectangle, Shell, Size, Widget};

#[cfg(target_os = "macos")]
use std::cell::Cell;
#[cfg(target_os = "macos")]
use std::rc::Rc;

pub struct Webview {
    url: Url,
    width: Length,
    height: Length,
}

pub type Url = String;

impl Webview {
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            width: Length::Fill,
            height: Length::Fill,
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
}

enum State {
    New,
    Ready {
        webview: wry::WebView,
        bounds: Rectangle,
        #[cfg(target_os = "macos")]
        cursor: Rc<Cell<Option<String>>>,
        #[cfg(target_os = "macos")]
        interaction: mouse::Interaction,
    },
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Webview
where
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

            let State::Ready {
                webview, bounds, ..
            } = state
            else {
                let bounds = layout.bounds();
                #[cfg(target_os = "macos")]
                let cursor = Rc::new(Cell::new(None));

                let webview = wry::WebViewBuilder::new()
                    .with_url(&self.url)
                    .with_bounds(into_rect(bounds));

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
                    bounds,
                    #[cfg(target_os = "macos")]
                    cursor,
                    #[cfg(target_os = "macos")]
                    interaction: mouse::Interaction::None,
                };

                return;
            };

            let new_bounds = layout.bounds();

            if *bounds != new_bounds {
                let _ = webview.set_bounds(into_rect(new_bounds));
                *bounds = new_bounds;
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

impl<'a, Message, Theme, Renderer> From<Webview> for Element<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    fn from(webview: Webview) -> Self {
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
