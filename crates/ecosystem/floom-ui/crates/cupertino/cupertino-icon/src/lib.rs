use floom::prelude::*;
use floom::reactive::{RwSignal, SignalGet};
use floom::views::{Decorators, svg};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon::IconProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

pub use floom_widget_cupertino_icon_paths::{cupertino_icon_path, cupertino_icon_svg};

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoIconStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub size: f32,
}

pub struct CupertinoIcon {
    props: IconProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoIcon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            props: IconProps {
                name: name.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = Some(size);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self.props.decorative = false;
        self
    }

    pub fn decorative(mut self, decorative: bool) -> Self {
        self.props.decorative = decorative;
        self
    }

    pub fn props(&self) -> &IconProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoIconStyle {
        let theme = self.theme.get();
        let ComponentTokens::Icon(tokens) = theme.component(ComponentId::Icon) else {
            unreachable!("ComponentId::Icon must resolve to IconTokens")
        };
        CupertinoIconStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().on_surface,
            size: self.props.size.unwrap_or(tokens.size),
        }
    }

    /// Build a renderable Cupertino icon.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let name = self.props.name.clone();
        let size = self.props.size;
        svg(move || cupertino_icon_svg(&name)).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Icon(tokens) = t.component(ComponentId::Icon) else {
                unreachable!()
            };
            let sz = size.unwrap_or(tokens.size);
            s.size(sz, sz)
                .color(t.colors().on_surface_variant.to_peniko())
        })
    }
}

// ---------------------------------------------------------------------------
// The Floom 2.0 surface, added without removing the v1 one
// ---------------------------------------------------------------------------
//
// I first *replaced* this file with a 2.0-only version, and that broke five crates that
// are still v1: `cupertino-alert`, `cupertino-dialog`, `fluent-alert`, `fluent-dialog`
// and `fluent-stepper` all call `CupertinoIcon::new(..).view()`. The floem-ui workspace went
// from three error crates to eight.
//
// The mistake was treating "this crate is being ported" as "this crate's consumers have
// been ported". They have not — the port proceeds one shape at a time and the v1 tree
// is still there underneath. So the fix is to **add** a 2.0 surface and leave the v1 one
// alone, which is what the porting rule actually says: never reintroduce a v1 `floom`
// dependency into a ported crate, not never keep one in a crate that is still v1.
//
// The v1 widget below stays for its v1 callers. The three functions above it are what a
// ported kit uses, and note that they resolve **no theme at all** — which is the point:
// 2.0's icon is a widget the caller holds and restyles, not a builder that resolves a
// design system on the caller's behalf.

/// The box every icon in this set is authored in.
///
/// Named rather than inlined, because it is a property of the *data*: the table, the
/// engine widget built from it, and the test that checks the fallback all read it.
pub const VIEW_BOX: floom_types::Rect = floom_types::Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

/// The named icon, as an engine `VectorIcon`.
///
/// Total and unwrap-free: an unknown name gives the set's fallback, because that is
/// what the path table returns and inventing a second notion of "missing" here would be
/// a second answer to the same question.
pub fn cupertino_icon(name: &str) -> floom_widgets::VectorIcon {
    // `from_path_data` rather than `new(parse_path_data(..))`: the engine already has
    // the pair, and a kit that spells it the long way is a kit whose icon table and
    // every other icon in the workspace are parsed by two different-looking routes.
    floom_widgets::VectorIcon::from_path_data(cupertino_icon_path(name), VIEW_BOX)
}
