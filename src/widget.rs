mod typewriter;

#[cfg(feature = "rand")]
mod diffused_text;

#[cfg(feature = "geometry")]
mod dynamic_text;

pub use typewriter::Typewriter;

#[cfg(feature = "rand")]
pub use diffused_text::DiffusedText;

#[cfg(feature = "geometry")]
pub use dynamic_text::DynamicText;

use crate::core;

pub fn typewriter<'a, Theme, Renderer>(
    fragment: impl core::text::IntoFragment<'a>,
) -> Typewriter<'a, Theme, Renderer>
where
    Theme: core::widget::text::Catalog,
    Renderer: core::text::Renderer,
{
    Typewriter::new(fragment)
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
