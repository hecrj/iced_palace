pub mod widget;

use iced_core as core;

#[cfg(feature = "macros")]
pub mod debug {
    pub use iced_palace_macros::time;
}
