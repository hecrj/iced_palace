#[cfg(feature = "rand")]
mod diffused_text;

#[cfg(feature = "geometry")]
mod dynamic_text;

#[cfg(feature = "rand")]
pub use diffused_text::DiffusedText;

#[cfg(feature = "geometry")]
pub use dynamic_text::DynamicText;

#[cfg(feature = "rand")]
pub fn diffused_text<'a, Theme, Renderer>(
    fragment: impl crate::core::text::IntoFragment<'a>,
) -> DiffusedText<'a, Theme, Renderer>
where
    Theme: crate::core::widget::text::Catalog,
    Renderer: crate::core::text::Renderer,
{
    DiffusedText::new(fragment)
}

#[cfg(feature = "geometry")]
pub fn dynamic_text<'a, Theme, Renderer>(
    fragment: impl crate::core::text::IntoFragment<'a>,
) -> DynamicText<'a, Theme, Renderer>
where
    Theme: crate::core::widget::text::Catalog,
    Renderer: crate::core::text::Renderer + iced_widget::graphics::geometry::Renderer,
{
    DynamicText::new(fragment)
}
