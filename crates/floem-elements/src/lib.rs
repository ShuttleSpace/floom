#[cfg(not(any(feature = "material", feature = "cupertino", feature = "fluent")))]
compile_error!("Enable exactly one Design System feature: material, cupertino, or fluent");

#[cfg(any(
    all(feature = "material", feature = "cupertino"),
    all(feature = "material", feature = "fluent"),
    all(feature = "cupertino", feature = "fluent"),
))]
compile_error!("Only one Design System feature can be enabled at a time");

pub mod button;
pub mod card;
pub mod checkbox;
pub mod dialog;
pub mod menu;
pub mod switch;
pub mod text;
pub mod text_field;

pub mod prelude {
    pub use crate::button::*;
    pub use crate::card::*;
    pub use crate::checkbox::*;
    pub use crate::dialog::*;
    pub use crate::menu::*;
    pub use crate::switch::*;
    pub use crate::text::*;
    pub use crate::text_field::*;
}
