mod diffused_text;

#[cfg(feature = "geometry")]
mod dynamic_text;

pub use diffused_text::DiffusedText;

#[cfg(feature = "geometry")]
pub use dynamic_text::DynamicText;

use crate::core::text;
use crate::core::widget;

pub fn diffused_text<'a, Theme, Renderer>(
    fragment: impl text::IntoFragment<'a>,
) -> DiffusedText<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    DiffusedText::new(fragment)
}

#[cfg(feature = "geometry")]
pub fn dynamic_text<'a, Theme, Renderer>(
    fragment: impl text::IntoFragment<'a>,
) -> DynamicText<'a, Theme, Renderer>
where
    Theme: widget::text::Catalog,
    Renderer: text::Renderer,
{
    DynamicText::new(fragment)
}
