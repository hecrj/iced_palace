use crate::core;
use crate::core::alignment;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text::{self, Fragment, Paragraph, Text};
use crate::core::time::{Duration, Instant, milliseconds};
use crate::core::widget;
use crate::core::widget::text::Format;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Alignment, Clipboard, Color, Element, Event, Length, Pixels, Rectangle, Shell, Size, Widget,
};

#[derive(Debug)]
pub struct Typewriter<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    fragment: Fragment<'a>,
    format: Format<Renderer::Font>,
    class: Theme::Class<'a>,
    speed: Duration,
}

impl<'a, Theme, Renderer> Typewriter<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    pub fn new(fragment: impl core::text::IntoFragment<'a>) -> Self {
        Self {
            fragment: fragment.into_fragment(),
            format: Format::default(),
            class: Theme::default(),
            speed: Duration::from_millis(20),
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.format.size = Some(size.into());
        self
    }

    pub fn line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.format.line_height = line_height.into();
        self
    }

    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.format.font = Some(font.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.format.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.format.height = height.into();
        self
    }

    pub fn align_x(mut self, alignment: impl Into<text::Alignment>) -> Self {
        self.format.align_x = alignment.into();
        self
    }

    pub fn align_y(mut self, alignment: impl Into<alignment::Vertical>) -> Self {
        self.format.align_y = alignment.into();
        self
    }

    pub fn center(self) -> Self {
        self.align_x(Alignment::Center).align_y(Alignment::Center)
    }

    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.format.shaping = shaping;
        self
    }

    pub fn wrapping(mut self, wrapping: text::Wrapping) -> Self {
        self.format.wrapping = wrapping;
        self
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> widget::text::Style + 'a) -> Self
    where
        Theme::Class<'a>: From<widget::text::StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as widget::text::StyleFn<'a, Theme>).into();
        self
    }

    pub fn color(self, color: impl Into<Color>) -> Self
    where
        Theme::Class<'a>: From<widget::text::StyleFn<'a, Theme>>,
    {
        self.color_maybe(Some(color))
    }

    pub fn color_maybe(self, color: Option<impl Into<Color>>) -> Self
    where
        Theme::Class<'a>: From<widget::text::StyleFn<'a, Theme>>,
    {
        let color = color.map(Into::into);

        self.style(move |_theme| widget::text::Style { color })
    }

    pub fn very_quick(self) -> Self {
        self.speed(milliseconds(10))
    }

    pub fn quick(self) -> Self {
        self.speed(milliseconds(20))
    }

    pub fn slow(self) -> Self {
        self.speed(milliseconds(40))
    }

    pub fn very_slow(self) -> Self {
        self.speed(milliseconds(80))
    }

    pub fn speed(mut self, char_rate: impl Into<Duration>) -> Self {
        self.speed = char_rate.into();
        self
    }
}

/// The internal state of a [`Text`] widget.
pub struct State<P: text::Paragraph> {
    text: text::paragraph::Plain<P>,
    animation: Animation<P>,
}

enum Animation<P: text::Paragraph> {
    Ticking { text: P, start: Option<Instant> },
    Done,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Typewriter<'_, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
    Renderer::Paragraph: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            text: text::paragraph::Plain::<Renderer::Paragraph>::default(),
            animation: Animation::Ticking {
                text: Renderer::Paragraph::default(),
                start: None,
            },
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.format.width,
            height: self.format.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = &mut tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        let has_changed = state.text.content() != self.fragment;

        let node = widget::text::layout(
            &mut state.text,
            renderer,
            limits,
            &self.fragment,
            self.format,
        );

        if has_changed {
            let text = Text {
                content: "",
                ..state.text.as_text()
            };

            state.animation = Animation::Ticking {
                text: Renderer::Paragraph::with_text(text),
                start: None,
            };
        }

        node
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let style = theme.style(&self.class);

        let paragraph = match &state.animation {
            Animation::Ticking { text, .. } => text,
            Animation::Done => state.text.raw(),
        };

        let position = layout.bounds().anchor(
            Size::new(paragraph.min_width(), state.text.min_height()),
            self.format.align_x,
            self.format.align_y,
        );

        renderer.fill_paragraph(
            paragraph,
            position,
            style.color.unwrap_or(defaults.text_color),
            *viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if layout.bounds().intersection(viewport).is_none() {
            return;
        }

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

            match &mut state.animation {
                Animation::Ticking { text, start } => {
                    let start = match start {
                        Some(start) => *start,
                        None => {
                            *start = Some(*now);
                            *now
                        }
                    };

                    let tick_rate = self.speed.as_millis() as f32;
                    let tick = ((*now - start).as_millis() as f32 / tick_rate) as usize;

                    let total_chars = self.fragment.chars().count();

                    if tick >= total_chars {
                        state.animation = Animation::Done;
                    } else {
                        let truncated: String = self.fragment.chars().take(tick).collect();

                        *text = Renderer::Paragraph::with_text(Text {
                            content: truncated.trim(),
                            ..state.text.as_text()
                        });

                        shell.request_redraw_at(*now + Duration::from_millis(tick_rate as u64));
                    }
                }
                Animation::Done => {}
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Typewriter<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: widget::text::Catalog + 'a,
    Renderer: text::Renderer + 'a,
    Renderer::Paragraph: Clone,
{
    fn from(text: Typewriter<'a, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
