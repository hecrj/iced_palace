use crate::core;
use crate::core::alignment;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::time::{Duration, Instant, milliseconds};
use crate::core::widget;
use crate::core::widget::text::{Catalog, Format, Style, StyleFn};
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Alignment, Clipboard, Color, Element, Event, Length, Pixels, Rectangle, Shell, Size, Widget,
};

#[derive(Debug)]
pub struct DiffusedText<'a, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fragment: core::text::Fragment<'a>,
    format: Format<Renderer::Font>,
    class: Theme::Class<'a>,
    duration: Duration,
    tick_rate: u64,
}

impl<'a, Theme, Renderer> DiffusedText<'a, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    pub fn new(fragment: impl core::text::IntoFragment<'a>) -> Self {
        Self {
            fragment: fragment.into_fragment(),
            format: Format::default(),
            class: Theme::default(),
            duration: Duration::from_millis(200),
            tick_rate: 50,
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

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    pub fn color(self, color: impl Into<Color>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.color_maybe(Some(color))
    }

    pub fn color_maybe(self, color: Option<impl Into<Color>>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        let color = color.map(Into::into);

        self.style(move |_theme| Style { color })
    }

    pub fn very_quick(self) -> Self {
        self.duration(milliseconds(100))
    }

    pub fn quick(self) -> Self {
        self.duration(milliseconds(200))
    }

    pub fn slow(self) -> Self {
        self.duration(milliseconds(400))
    }

    pub fn very_slow(self) -> Self {
        self.duration(milliseconds(500))
    }

    pub fn duration(mut self, duration: impl Into<Duration>) -> Self {
        self.duration = duration.into();
        self
    }

    pub fn tick_rate(mut self, tick_rate: impl Into<Duration>) -> Self {
        self.tick_rate = tick_rate.into().as_millis() as u64;
        self
    }
}

/// The internal state of a [`Text`] widget.
#[derive(Debug)]
pub struct State<P: text::Paragraph> {
    content: String,
    internal: widget::text::State<P>,
    animation: Animation,
}

#[derive(Debug)]
enum Animation {
    Ticking {
        fragment: String,
        ticks: u64,
        next_redraw: Instant,
    },
    Done,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DiffusedText<'_, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            content: String::new(),
            internal: widget::text::State::<Renderer::Paragraph>::default(),
            animation: Animation::Ticking {
                fragment: String::new(),
                ticks: 0,
                next_redraw: Instant::now(),
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

        if state.content != self.fragment {
            state.content = self.fragment.clone().into_owned();

            state.animation = Animation::Ticking {
                fragment: String::from("-"),
                ticks: 0,
                next_redraw: Instant::now(),
            };
        }

        let fragment = match &state.animation {
            Animation::Ticking { fragment, .. } => fragment,
            Animation::Done => self.fragment.as_ref(),
        };

        widget::text::layout(&mut state.internal, renderer, limits, fragment, self.format)
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

        widget::text::draw(
            renderer,
            defaults,
            layout.bounds(),
            state.internal.raw(),
            style,
            viewport,
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
        use rand::Rng;

        if layout.bounds().intersection(viewport).is_none() {
            return;
        }

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

            match &mut state.animation {
                Animation::Ticking {
                    fragment,
                    next_redraw,
                    ticks,
                } => {
                    if *next_redraw <= *now {
                        *ticks += 1;

                        let mut rng = rand::rng();
                        let progress = (self.fragment.len() as f32
                            / self.duration.as_millis() as f32
                            * (*ticks * self.tick_rate) as f32)
                            as usize;

                        if progress >= self.fragment.len() {
                            state.animation = Animation::Done;
                            shell.invalidate_layout();

                            return;
                        }

                        *fragment = self
                            .fragment
                            .chars()
                            .take(progress)
                            .chain(self.fragment.chars().skip(progress).map(|c| {
                                if c.is_whitespace() || c == '-' {
                                    c
                                } else {
                                    rng.random_range('a'..='z')
                                }
                            }))
                            .collect::<String>();

                        *next_redraw = *now + Duration::from_millis(self.tick_rate);

                        shell.invalidate_layout();
                    }

                    shell.request_redraw_at(*next_redraw);
                }
                Animation::Done => {}
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<DiffusedText<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: widget::text::Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(text: DiffusedText<'a, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
