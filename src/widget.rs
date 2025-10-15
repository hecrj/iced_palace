mod ellipsized_text;
mod typewriter;

#[cfg(feature = "rand")]
mod diffused_text;

#[cfg(feature = "geometry")]
mod dynamic_text;

pub use ellipsized_text::EllipsizedText;
pub use typewriter::Typewriter;

#[cfg(feature = "rand")]
pub use diffused_text::DiffusedText;

#[cfg(feature = "geometry")]
pub use dynamic_text::DynamicText;

use crate::core;
use crate::core::border;
use crate::core::{Alignment, Color, Element, Length};
use iced_widget::{container, row, slider, space, stack, text};

use std::ops::RangeInclusive;

pub fn typewriter<'a, Theme, Renderer>(
    fragment: impl core::text::IntoFragment<'a>,
) -> Typewriter<'a, Theme, Renderer>
where
    Theme: core::widget::text::Catalog,
    Renderer: core::text::Renderer,
{
    Typewriter::new(fragment)
}

pub fn ellipsized_text<'a, Theme, Renderer>(
    fragment: impl core::text::IntoFragment<'a>,
) -> EllipsizedText<'a, Theme, Renderer>
where
    Theme: core::widget::text::Catalog,
    Renderer: core::text::Renderer,
{
    EllipsizedText::new(fragment)
}

#[cfg(feature = "rand")]
pub fn diffused_text<'a, Theme, Renderer>(
    fragment: impl core::text::IntoFragment<'a>,
) -> DiffusedText<'a, Theme, Renderer>
where
    Theme: core::widget::text::Catalog,
    Renderer: core::text::Renderer,
{
    DiffusedText::new(fragment)
}

#[cfg(feature = "geometry")]
pub fn dynamic_text<'a, Theme, Renderer>(
    fragment: impl core::text::IntoFragment<'a>,
) -> DynamicText<'a, Theme, Renderer>
where
    Theme: core::widget::text::Catalog,
    Renderer: core::text::Renderer + iced_widget::graphics::geometry::Renderer,
{
    DynamicText::new(fragment)
}

pub fn labeled_slider<'a, T, Message, Renderer>(
    label: impl text::IntoFragment<'a>,
    (range, step): (RangeInclusive<T>, T),
    current: T,
    on_change: impl Fn(T) -> Message + 'a,
    to_string: impl Fn(&T) -> String,
) -> Element<'a, Message, core::Theme, Renderer>
where
    T: Copy + PartialOrd + Into<f64> + From<u8> + num_traits::FromPrimitive + 'static,
    Message: Clone + 'a,
    Renderer: core::text::Renderer + 'a,
{
    stack![
        container(
            slider(range, current, on_change)
                .step(step)
                .width(Length::Fill)
                .height(24)
                .style(|theme: &core::Theme, status| {
                    let palette = theme.extended_palette();

                    slider::Style {
                        rail: slider::Rail {
                            backgrounds: (
                                match status {
                                    slider::Status::Active | slider::Status::Dragged => {
                                        palette.background.strongest.color
                                    }
                                    slider::Status::Hovered => palette.background.stronger.color,
                                }
                                .into(),
                                Color::TRANSPARENT.into(),
                            ),
                            width: 24.0,
                            border: border::rounded(2),
                        },
                        handle: slider::Handle {
                            shape: slider::HandleShape::Circle { radius: 0.0 },
                            background: Color::TRANSPARENT.into(),
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                    }
                })
        )
        .style(|theme| container::Style::default()
            .background(theme.extended_palette().background.weak.color)
            .border(border::rounded(2))),
        row![
            text(label).size(14).style(|theme: &core::Theme| {
                text::Style {
                    color: Some(theme.extended_palette().background.weak.text),
                }
            }),
            space::horizontal(),
            text(to_string(&current)).size(14)
        ]
        .padding([0, 10])
        .height(Length::Fill)
        .align_y(Alignment::Center),
    ]
    .into()
}
