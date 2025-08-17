use crate::core;
use crate::core::alignment;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text::{self, Fragment, Hit, Paragraph};
use crate::core::widget;
use crate::core::widget::text::Format;
use crate::core::widget::tree::{self, Tree};
use crate::core::{Alignment, Color, Element, Length, Pixels, Point, Rectangle, Size, Widget};

#[derive(Debug)]
pub struct EllipsizedText<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    fragment: Fragment<'a>,
    format: Format<Renderer::Font>,
    class: Theme::Class<'a>,
}

impl<'a, Theme, Renderer> EllipsizedText<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    pub fn new(fragment: impl core::text::IntoFragment<'a>) -> Self {
        Self {
            fragment: fragment.into_fragment(),
            format: Format::default(),
            class: Theme::default(),
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
}

struct State<P: text::Paragraph> {
    original: text::paragraph::Plain<P>,
    ellipsis: text::paragraph::Plain<P>,
    ellipsized: text::paragraph::Plain<P>,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for EllipsizedText<'_, Theme, Renderer>
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
            original: text::paragraph::Plain::<Renderer::Paragraph>::default(),
            ellipsis: text::paragraph::Plain::<Renderer::Paragraph>::default(),
            ellipsized: text::paragraph::Plain::<Renderer::Paragraph>::default(),
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.format.width,
            height: self.format.height,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = &mut tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let format = self.format;

        layout::sized(limits, format.width, format.height, |limits| {
            let bounds = limits.max();

            let size = format.size.unwrap_or_else(|| renderer.default_size());
            let font = format.font.unwrap_or_else(|| renderer.default_font());

            let changed = state.original.update(text::Text {
                content: &self.fragment,
                bounds,
                size,
                line_height: format.line_height,
                font,
                align_x: format.align_x,
                align_y: format.align_y,
                shaping: format.shaping,
                wrapping: format.wrapping,
            });

            if changed {
                state.ellipsis.update(text::Text {
                    content: "...",
                    bounds: Size::INFINITE,
                    size,
                    line_height: format.line_height,
                    font,
                    shaping: format.shaping,
                    wrapping: text::Wrapping::None,
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Center,
                });

                let min_bounds = state.original.min_bounds().min(bounds);
                let y_offset = self.format.line_height.to_absolute(size).0 / 2.0;

                let hit = state.original.raw().hit_test(Point {
                    x: min_bounds.width,
                    y: min_bounds.height - y_offset,
                });

                match hit {
                    Some(Hit::CharOffset(offset)) if offset < self.fragment.len() => {
                        let Hit::CharOffset(offset) = state
                            .original
                            .raw()
                            .hit_test(Point {
                                x: min_bounds.width - state.ellipsis.min_width() * 1.25,
                                y: min_bounds.height - y_offset,
                            })
                            .unwrap_or(Hit::CharOffset(offset));

                        state.ellipsized.update(text::Text {
                            content: &format!(
                                "{}...",
                                &self.fragment[..offset].trim().trim_end_matches([',', '.'])
                            ),
                            bounds,
                            size,
                            line_height: format.line_height,
                            font,
                            align_x: format.align_x,
                            align_y: format.align_y,
                            shaping: format.shaping,
                            wrapping: format.wrapping,
                        });
                    }
                    _ => {
                        state.ellipsized = state.original.clone();
                    }
                }
            }

            state.ellipsized.min_bounds()
        })
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

        let position = layout.bounds().anchor(
            state.ellipsized.min_bounds(),
            self.format.align_x,
            self.format.align_y,
        );

        renderer.fill_paragraph(
            state.ellipsized.raw(),
            position,
            style.color.unwrap_or(defaults.text_color),
            *viewport,
        );
    }
}

impl<'a, Message, Theme, Renderer> From<EllipsizedText<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: widget::text::Catalog + 'a,
    Renderer: text::Renderer + 'a,
    Renderer::Paragraph: Clone,
{
    fn from(text: EllipsizedText<'a, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
