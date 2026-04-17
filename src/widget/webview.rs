use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::widget;
use crate::core::window;
use crate::core::{Element, Event, Layout, Length, Rectangle, Shell, Size, Widget};

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

            let State::Ready { webview, bounds } = state else {
                let bounds = layout.bounds();

                let webview = wry::WebViewBuilder::new()
                    .with_url(&self.url)
                    .with_bounds(into_rect(bounds))
                    .build_as_child(&shell.window())
                    .expect("start webview");

                *state = State::Ready { webview, bounds };

                return;
            };

            let new_bounds = layout.bounds();

            if *bounds != new_bounds {
                let _ = webview.set_bounds(into_rect(new_bounds));
                *bounds = new_bounds;
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
