mod diffused_text;

pub use diffused_text::DiffusedText;

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
