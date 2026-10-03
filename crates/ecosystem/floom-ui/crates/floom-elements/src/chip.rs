pub use floom_core_theme::component_tokens::ChipTokens;
pub use floom_core_theme::props::chip::ChipProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_chip::CupertinoChip as Chip;
#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_chip::CupertinoChipStyle as ChipStyle;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_chip::FluentChip as Chip;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_chip::FluentChipStyle as ChipStyle;
#[cfg(feature = "material")]
pub use floom_widget_md_chip::MdChip as Chip;
#[cfg(feature = "material")]
pub use floom_widget_md_chip::MdChipStyle as ChipStyle;
