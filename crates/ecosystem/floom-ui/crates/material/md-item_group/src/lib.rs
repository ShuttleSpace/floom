
use floom_core::Axis;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_ui_composites::{ContentPlacement, SelectGroup, SelectGroupStyle, SelectionMode};

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ItemGroupTokens};
use floom_core_theme::props::item_group::ItemGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

pub type RowBuilder =
    std::sync::Arc<dyn Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>;

fn item_group_tokens(theme: &MultiTheme) -> ItemGroupTokens {
    match theme.component(ComponentId::ItemGroup) {
        ComponentTokens::ItemGroup(t) => t,
        _ => unreachable!("ComponentId::ItemGroup must resolve to ItemGroupTokens"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdItemGroupStyle {
    pub design_system: &'static str,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub primary: Color,
    pub outline: Color,
    pub gap: f32,
    pub pad: f32,
    pub radius: f32,
}

/// One row, as it was declared.
struct ItemRow {
    label: Option<String>,
    content: RowBuilder,
}

pub struct MdItemGroup {
    props: ItemGroupProps,
    theme: RwSignal<MultiTheme>,
    rows: Vec<ItemRow>,
    initial: Option<usize>,
}

impl MdItemGroup {
    pub fn new() -> Self {
        Self {
            props: ItemGroupProps::default(),
            theme: theme_signal(),
            rows: Vec::new(),
            initial: None,
        }
    }

    /// Append a named row.
    pub fn row(
        mut self,
        label: impl Into<String>,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(ItemRow {
            label: Some(label.into()),
            content: std::sync::Arc::new(content),
        });
        self
    }

    /// Append a row with no accessible name.
    pub fn row_unlabelled(
        mut self,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(ItemRow {
            label: None,
            content: std::sync::Arc::new(content),
        });
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.initial = Some(selected);
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    pub fn props(&self) -> &ItemGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdItemGroupStyle {
        let theme = self.theme.get();
        let tokens = item_group_tokens(&theme);
        MdItemGroupStyle {
            design_system: "material",
            surface_variant: theme.colors().surface_variant,
            on_surface: theme.colors().on_surface,
            primary_container: theme.colors().primary_container,
            on_primary_container: theme.colors().on_primary_container,
            primary: theme.colors().primary,
            outline: theme.colors().outline,
            gap: tokens.gap,
            pad: tokens.pad,
            radius: tokens.radius,
        }
    }

    pub fn select_group_style(&self) -> SelectGroupStyle {
        let style = self.resolved_style();
        SelectGroupStyle {
            gap: style.gap,
            axis: Axis::Vertical,
            height: None,
            pad: style.pad,
            content_placement: ContentPlacement::Top,
            toggles_off_in_single_mode: false,
            mandatory_in_multiple_mode: true,
            radius: style.radius,
            border_width: 1.0,
            background: to_paint(style.surface_variant),
            foreground: to_paint(style.on_surface),
            border_color: to_paint(style.outline),
            selected_background: to_paint(style.primary_container),
            selected_foreground: to_paint(style.on_primary_container),
            selected_border_color: to_paint(style.primary),
        }
    }

    pub fn foreground(&self, selected: bool) -> PaintColor {
        if selected {
            to_paint(self.resolved_style().on_primary_container)
        } else {
            to_paint(self.resolved_style().on_surface)
        }
    }

    pub fn padding(&self) -> Insets {
        let pad = f64::from(self.resolved_style().pad);
        Insets {
            left: pad,
            right: pad,
            top: pad / 2.0,
            bottom: pad / 2.0,
        }
    }

    pub fn view(
        self,
        on_change_single: Option<impl Fn(usize) + Send + Sync + 'static>,
        on_change_multi: Option<impl Fn(Vec<usize>) + Send + Sync + 'static>,
    ) -> SelectGroup {
        let mode = if self.props.multiple {
            SelectionMode::Multiple
        } else {
            SelectionMode::Single
        };
        let single = on_change_single.map(std::sync::Arc::new);
        let multi = on_change_multi.map(std::sync::Arc::new);

        let mut group = SelectGroup::new()
            .mode(mode)
            .mandatory(self.props.mandatory)
            .style(self.select_group_style());
        if let Some(index) = self.initial {
            group = group.selected([index]);
        }
        group = group.on_change(move |next| {
            if let Some(single) = &single {
                if let Some(last) = next.last() {
                    single(*last);
                }
            }
            if let Some(multi) = &multi {
                multi(next.clone());
            }
        });
        for row in self.rows {
            let content = row.content;
            group = match row.label {
                Some(label) => group.row(label, move || content()),
                None => group.row_unlabelled(move || content()),
            };
        }
        group
    }
}

pub fn selected(group: &SelectGroup) -> Option<usize> {
    group.selection().last().copied()
}

pub fn selected_multi(group: &SelectGroup) -> Vec<usize> {
    group.selection()
}

impl Default for MdItemGroup {
    fn default() -> Self {
        Self::new()
    }
}
