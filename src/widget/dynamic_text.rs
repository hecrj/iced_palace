use crate::core;
use crate::core::alignment;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::text::paragraph;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::{
    Alignment, Color, Element, Font, Length, Pixels, Point, Rectangle, Size, Widget,
};

use iced_widget::canvas;
use iced_widget::graphics::geometry;

#[derive(Debug)]
pub struct DynamicText<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    fragment: core::text::Fragment<'a>,
    size: Option<Pixels>,
    line_height: text::LineHeight,
    width: Length,
    height: Length,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    font: Option<Renderer::Font>,
    shaping: text::Shaping,
    vectorial: bool,
    class: Theme::Class<'a>,
}

impl<'a, Theme, Renderer> DynamicText<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub fn new(fragment: impl core::text::IntoFragment<'a>) -> Self {
        Self {
            fragment: fragment.into_fragment(),
            size: None,
            line_height: text::LineHeight::default(),
            font: None,
            width: Length::Shrink,
            height: Length::Shrink,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Basic,
            vectorial: false,
            class: Theme::default(),
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
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

    pub fn align_x(mut self, alignment: impl Into<text::Alignment>) -> Self {
        self.align_x = alignment.into();
        self
    }

    pub fn align_y(mut self, alignment: impl Into<alignment::Vertical>) -> Self {
        self.align_y = alignment.into();
        self
    }

    pub fn center(self) -> Self {
        self.align_x(Alignment::Center).align_y(Alignment::Center)
    }

    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    pub fn vectorial(mut self, vectorial: bool) -> Self {
        self.vectorial = vectorial;
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

/// The internal state of a [`Text`] widget.
pub struct State<Renderer>
where
    Renderer: text::Renderer + geometry::Renderer + 'static,
{
    text: paragraph::Plain<Renderer::Paragraph>,
    geometry: canvas::Cache<Renderer>,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for DynamicText<'_, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer<Font = Font> + geometry::Renderer + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            text: paragraph::Plain::<Renderer::Paragraph>::default(),
            geometry: canvas::Cache::<Renderer>::new(),
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = &mut tree.state.downcast_mut::<State<Renderer>>();

        layout::sized(limits, self.width, self.height, |limits| {
            let bounds = limits.max();

            let size = self.size.unwrap_or_else(|| renderer.default_size());
            let font = self.font.unwrap_or_else(|| renderer.default_font());

            let changed = state.text.update(text::Text {
                content: &self.fragment,
                bounds,
                size,
                line_height: self.line_height,
                font,
                align_x: self.align_x,
                align_y: self.align_y,
                shaping: self.shaping,
                wrapping: text::Wrapping::default(),
                hint_factor: None,
            });

            if changed {
                state.geometry.clear();
            }

            state.text.min_bounds()
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
        let state = tree.state.downcast_ref::<State<Renderer>>();
        let style = theme.style(&self.class);

        let text_bounds = state.text.min_bounds();
        let text_position = {
            let x = match self.align_x {
                text::Alignment::Default | text::Alignment::Left | text::Alignment::Justified => {
                    0.0
                }
                text::Alignment::Center => text_bounds.width / 2.0,
                text::Alignment::Right => text_bounds.width,
            };

            let y = match self.align_y {
                alignment::Vertical::Top => 0.0,
                alignment::Vertical::Center => text_bounds.height / 2.0,
                alignment::Vertical::Bottom => text_bounds.height,
            };

            Point::new(x, y)
        };

        let geometry = state.geometry.draw(renderer, text_bounds, |frame| {
            canvas::Text {
                content: self.fragment.clone().into_owned(),
                position: text_position,
                max_width: text_bounds.width,
                color: style.color.unwrap_or(defaults.text_color),
                size: self.size.unwrap_or(renderer.default_size()),
                line_height: self.line_height,
                font: self.font.unwrap_or(renderer.default_font()),
                align_x: self.align_x,
                align_y: self.align_y,
                shaping: self.shaping,
            }
            .draw_with(|glyph, color| {
                frame.fill(&glyph, color);
            });
        });

        let position = layout
            .bounds()
            .anchor(state.text.min_bounds(), self.align_x, self.align_y);

        if self.vectorial {
            renderer.with_translation(position - Point::ORIGIN, |renderer| {
                renderer.draw_geometry(geometry);
            });
        } else {
            let style = theme.style(&self.class);

            renderer.fill_paragraph(
                state.text.raw(),
                position,
                style.color.unwrap_or(defaults.text_color),
                *viewport,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<DynamicText<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: widget::text::Catalog + 'a,
    Renderer: text::Renderer<Font = Font> + geometry::Renderer + 'static,
{
    fn from(text: DynamicText<'a, Theme, Renderer>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
