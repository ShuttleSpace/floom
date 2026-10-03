use crate::component_tokens::material::MaterialComponentTokens;
use crate::component_tokens::{ComponentId, ComponentTokens};
use crate::global_theme::{DesignSystem, current_design_system, is_dark};
use crate::tokens::color::{Color, ColorTokens};
use crate::tokens::motion::MotionTokens;
use crate::tokens::radius::RadiusTokens;
use crate::tokens::shadow::ElevationTokens;
use crate::tokens::spacing::SpacingTokens;
use crate::tokens::typography::{FontWeight, TextStyle, TypeScale, TypographyTokens};

pub trait Theme: Clone + Send + Sync + 'static {
    fn colors(&self) -> &ColorTokens;
    fn spacing(&self) -> &SpacingTokens;
    fn typography(&self) -> &TypographyTokens;
    fn radius(&self) -> &RadiusTokens;
    fn shadow(&self) -> &ElevationTokens;
    fn motion(&self) -> &MotionTokens;
    fn component(&self, id: ComponentId) -> ComponentTokens;
}

#[derive(Debug, Clone)]
pub struct MaterialTheme {
    colors: ColorTokens,
    colors_dark: ColorTokens,
    spacing: SpacingTokens,
    typography: TypographyTokens,
    radius: RadiusTokens,
    shadow: ElevationTokens,
    motion: MotionTokens,
    component_tokens: MaterialComponentTokens,
}

impl MaterialTheme {
    pub fn new() -> Self {
        Self {
            colors: ColorTokens {
                primary: Color::rgb(103, 80, 164),
                on_primary: Color::rgb(255, 255, 255),
                primary_container: Color::rgb(234, 221, 255),
                on_primary_container: Color::rgb(33, 0, 93),
                secondary: Color::rgb(98, 91, 113),
                on_secondary: Color::rgb(255, 255, 255),
                secondary_container: Color::rgb(232, 222, 248),
                on_secondary_container: Color::rgb(29, 25, 43),
                tertiary: Color::rgb(125, 82, 96),
                on_tertiary: Color::rgb(255, 255, 255),
                error: Color::rgb(179, 38, 30),
                on_error: Color::rgb(255, 255, 255),
                error_container: Color::rgb(249, 222, 220),
                on_error_container: Color::rgb(65, 14, 11),
                surface: Color::rgb(254, 247, 255),
                on_surface: Color::rgb(29, 27, 32),
                surface_variant: Color::rgb(231, 224, 236),
                on_surface_variant: Color::rgb(73, 69, 79),
                outline: Color::rgb(121, 116, 126),
                outline_variant: Color::rgb(202, 196, 208),
                background: Color::rgb(254, 247, 255),
                on_background: Color::rgb(29, 27, 32),
                scrim: Color::rgba(0, 0, 0, 128),
            },
            colors_dark: ColorTokens {
                primary: Color::rgb(208, 188, 255),
                on_primary: Color::rgb(55, 30, 110),
                primary_container: Color::rgb(79, 55, 139),
                on_primary_container: Color::rgb(234, 221, 255),
                secondary: Color::rgb(204, 194, 220),
                on_secondary: Color::rgb(51, 45, 65),
                secondary_container: Color::rgb(73, 68, 88),
                on_secondary_container: Color::rgb(232, 222, 248),
                tertiary: Color::rgb(239, 184, 200),
                on_tertiary: Color::rgb(73, 37, 50),
                error: Color::rgb(255, 180, 171),
                on_error: Color::rgb(105, 0, 5),
                error_container: Color::rgb(147, 0, 10),
                on_error_container: Color::rgb(255, 218, 214),
                surface: Color::rgb(28, 27, 31),
                on_surface: Color::rgb(231, 224, 236),
                surface_variant: Color::rgb(73, 69, 79),
                on_surface_variant: Color::rgb(202, 196, 208),
                outline: Color::rgb(147, 143, 153),
                outline_variant: Color::rgb(73, 69, 79),
                background: Color::rgb(28, 27, 31),
                on_background: Color::rgb(231, 224, 236),
                scrim: Color::rgba(0, 0, 0, 128),
            },
            spacing: SpacingTokens::default(),
            typography: TypographyTokens {
                display: TypeScale {
                    large: TextStyle {
                        font_size: 57.0,
                        line_height: 64.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    medium: TextStyle {
                        font_size: 45.0,
                        line_height: 52.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    small: TextStyle {
                        font_size: 36.0,
                        line_height: 44.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                },
                headline: TypeScale {
                    large: TextStyle {
                        font_size: 32.0,
                        line_height: 40.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    medium: TextStyle {
                        font_size: 28.0,
                        line_height: 36.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    small: TextStyle {
                        font_size: 24.0,
                        line_height: 32.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                },
                title: TypeScale {
                    large: TextStyle {
                        font_size: 22.0,
                        line_height: 28.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    medium: TextStyle {
                        font_size: 16.0,
                        line_height: 24.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Medium,
                    },
                    small: TextStyle {
                        font_size: 14.0,
                        line_height: 20.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Medium,
                    },
                },
                body: TypeScale {
                    large: TextStyle {
                        font_size: 16.0,
                        line_height: 24.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    medium: TextStyle {
                        font_size: 14.0,
                        line_height: 20.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                    small: TextStyle {
                        font_size: 12.0,
                        line_height: 16.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Regular,
                    },
                },
                label: TypeScale {
                    large: TextStyle {
                        font_size: 14.0,
                        line_height: 20.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Medium,
                    },
                    medium: TextStyle {
                        font_size: 12.0,
                        line_height: 16.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Medium,
                    },
                    small: TextStyle {
                        font_size: 11.0,
                        line_height: 16.0,
                        letter_spacing: 0.0,
                        font_weight: FontWeight::Medium,
                    },
                },
            },
            radius: RadiusTokens::default(),
            shadow: ElevationTokens::default(),
            motion: MotionTokens::default(),
            component_tokens: MaterialComponentTokens::new(),
        }
    }
}

impl Theme for MaterialTheme {
    fn colors(&self) -> &ColorTokens {
        if is_dark() { &self.colors_dark } else { &self.colors }
    }
    fn spacing(&self) -> &SpacingTokens {
        &self.spacing
    }
    fn typography(&self) -> &TypographyTokens {
        &self.typography
    }
    fn radius(&self) -> &RadiusTokens {
        &self.radius
    }
    fn shadow(&self) -> &ElevationTokens {
        &self.shadow
    }
    fn motion(&self) -> &MotionTokens {
        &self.motion
    }
    fn component(&self, id: ComponentId) -> ComponentTokens {
        self.component_tokens.get(id)
    }
}

#[derive(Debug, Clone)]
pub struct CupertinoTheme {
    colors: ColorTokens,
    colors_dark: ColorTokens,
    spacing: SpacingTokens,
    typography: TypographyTokens,
    radius: RadiusTokens,
    shadow: ElevationTokens,
    motion: MotionTokens,
    component_tokens: MaterialComponentTokens,
}

impl CupertinoTheme {
    pub fn new() -> Self {
        Self {
            colors: ColorTokens {
                primary: Color::rgb(0, 122, 255),
                on_primary: Color::rgb(255, 255, 255),
                primary_container: Color::rgb(229, 242, 255),
                on_primary_container: Color::rgb(0, 64, 133),
                secondary: Color::rgb(88, 86, 214),
                on_secondary: Color::rgb(255, 255, 255),
                secondary_container: Color::rgb(236, 235, 255),
                on_secondary_container: Color::rgb(43, 41, 128),
                tertiary: Color::rgb(255, 45, 85),
                on_tertiary: Color::rgb(255, 255, 255),
                error: Color::rgb(255, 59, 48),
                on_error: Color::rgb(255, 255, 255),
                error_container: Color::rgb(255, 235, 234),
                on_error_container: Color::rgb(120, 16, 12),
                surface: Color::rgb(255, 255, 255),
                on_surface: Color::rgb(28, 28, 30),
                surface_variant: Color::rgb(242, 242, 247),
                on_surface_variant: Color::rgb(99, 99, 102),
                outline: Color::rgb(198, 198, 200),
                outline_variant: Color::rgb(229, 229, 234),
                background: Color::rgb(242, 242, 247),
                on_background: Color::rgb(28, 28, 30),
                scrim: Color::rgba(0, 0, 0, 76),
            },
            colors_dark: ColorTokens {
                primary: Color::rgb(10, 132, 255),
                on_primary: Color::rgb(255, 255, 255),
                primary_container: Color::rgb(38, 83, 138),
                on_primary_container: Color::rgb(158, 200, 255),
                secondary: Color::rgb(94, 92, 230),
                on_secondary: Color::rgb(255, 255, 255),
                secondary_container: Color::rgb(67, 65, 160),
                on_secondary_container: Color::rgb(190, 188, 255),
                tertiary: Color::rgb(255, 69, 105),
                on_tertiary: Color::rgb(255, 255, 255),
                error: Color::rgb(255, 100, 97),
                on_error: Color::rgb(255, 255, 255),
                error_container: Color::rgb(130, 30, 26),
                on_error_container: Color::rgb(255, 178, 174),
                surface: Color::rgb(28, 28, 30),
                on_surface: Color::rgb(235, 235, 245),
                surface_variant: Color::rgb(44, 44, 46),
                on_surface_variant: Color::rgb(174, 174, 178),
                outline: Color::rgb(84, 84, 88),
                outline_variant: Color::rgb(58, 58, 60),
                background: Color::rgb(0, 0, 0),
                on_background: Color::rgb(235, 235, 245),
                scrim: Color::rgba(0, 0, 0, 140),
            },
            spacing: SpacingTokens::default(),
            typography: typography(17.0),
            radius: RadiusTokens {
                none: 0.0,
                xs: 4.0,
                sm: 8.0,
                md: 10.0,
                lg: 14.0,
                xl: 20.0,
                full: 9999.0,
            },
            shadow: ElevationTokens::default(),
            motion: MotionTokens::default(),
            component_tokens: MaterialComponentTokens::cupertino(),
        }
    }
}

impl Theme for CupertinoTheme {
    fn colors(&self) -> &ColorTokens {
        if is_dark() { &self.colors_dark } else { &self.colors }
    }
    fn spacing(&self) -> &SpacingTokens {
        &self.spacing
    }
    fn typography(&self) -> &TypographyTokens {
        &self.typography
    }
    fn radius(&self) -> &RadiusTokens {
        &self.radius
    }
    fn shadow(&self) -> &ElevationTokens {
        &self.shadow
    }
    fn motion(&self) -> &MotionTokens {
        &self.motion
    }
    fn component(&self, id: ComponentId) -> ComponentTokens {
        self.component_tokens.get(id)
    }
}

#[derive(Debug, Clone)]
pub struct FluentTheme {
    colors: ColorTokens,
    colors_dark: ColorTokens,
    spacing: SpacingTokens,
    typography: TypographyTokens,
    radius: RadiusTokens,
    shadow: ElevationTokens,
    motion: MotionTokens,
    component_tokens: MaterialComponentTokens,
}

impl FluentTheme {
    pub fn new() -> Self {
        Self {
            colors: ColorTokens {
                primary: Color::rgb(0, 120, 212),
                on_primary: Color::rgb(255, 255, 255),
                primary_container: Color::rgb(222, 236, 249),
                on_primary_container: Color::rgb(0, 69, 120),
                secondary: Color::rgb(96, 94, 92),
                on_secondary: Color::rgb(255, 255, 255),
                secondary_container: Color::rgb(243, 242, 241),
                on_secondary_container: Color::rgb(50, 49, 48),
                tertiary: Color::rgb(135, 100, 184),
                on_tertiary: Color::rgb(255, 255, 255),
                error: Color::rgb(196, 43, 28),
                on_error: Color::rgb(255, 255, 255),
                error_container: Color::rgb(253, 231, 233),
                on_error_container: Color::rgb(116, 0, 6),
                surface: Color::rgb(255, 255, 255),
                on_surface: Color::rgb(32, 31, 30),
                surface_variant: Color::rgb(243, 242, 241),
                on_surface_variant: Color::rgb(96, 94, 92),
                outline: Color::rgb(138, 136, 134),
                outline_variant: Color::rgb(225, 223, 221),
                background: Color::rgb(250, 249, 248),
                on_background: Color::rgb(32, 31, 30),
                scrim: Color::rgba(0, 0, 0, 102),
            },
            colors_dark: ColorTokens {
                primary: Color::rgb(96, 205, 255),
                on_primary: Color::rgb(0, 0, 0),
                primary_container: Color::rgb(0, 69, 120),
                on_primary_container: Color::rgb(173, 230, 255),
                secondary: Color::rgb(121, 119, 117),
                on_secondary: Color::rgb(0, 0, 0),
                secondary_container: Color::rgb(60, 58, 56),
                on_secondary_container: Color::rgb(232, 230, 228),
                tertiary: Color::rgb(175, 136, 223),
                on_tertiary: Color::rgb(0, 0, 0),
                error: Color::rgb(249, 129, 116),
                on_error: Color::rgb(0, 0, 0),
                error_container: Color::rgb(116, 0, 6),
                on_error_container: Color::rgb(255, 186, 182),
                surface: Color::rgb(32, 31, 30),
                on_surface: Color::rgb(249, 249, 249),
                surface_variant: Color::rgb(50, 49, 48),
                on_surface_variant: Color::rgb(223, 221, 219),
                outline: Color::rgb(128, 128, 128),
                outline_variant: Color::rgb(72, 70, 68),
                background: Color::rgb(32, 31, 30),
                on_background: Color::rgb(249, 249, 249),
                scrim: Color::rgba(0, 0, 0, 150),
            },
            spacing: SpacingTokens::default(),
            typography: typography(14.0),
            radius: RadiusTokens {
                none: 0.0,
                xs: 2.0,
                sm: 2.0,
                md: 4.0,
                lg: 6.0,
                xl: 8.0,
                full: 9999.0,
            },
            shadow: ElevationTokens::default(),
            motion: MotionTokens::default(),
            component_tokens: MaterialComponentTokens::fluent(),
        }
    }
}

impl Theme for FluentTheme {
    fn colors(&self) -> &ColorTokens {
        if is_dark() { &self.colors_dark } else { &self.colors }
    }
    fn spacing(&self) -> &SpacingTokens {
        &self.spacing
    }
    fn typography(&self) -> &TypographyTokens {
        &self.typography
    }
    fn radius(&self) -> &RadiusTokens {
        &self.radius
    }
    fn shadow(&self) -> &ElevationTokens {
        &self.shadow
    }
    fn motion(&self) -> &MotionTokens {
        &self.motion
    }
    fn component(&self, id: ComponentId) -> ComponentTokens {
        self.component_tokens.get(id)
    }
}

fn typography(body_medium: f32) -> TypographyTokens {
    TypographyTokens {
        display: TypeScale {
            large: text_style(body_medium + 24.0, FontWeight::Regular),
            medium: text_style(body_medium + 18.0, FontWeight::Regular),
            small: text_style(body_medium + 12.0, FontWeight::Regular),
        },
        headline: TypeScale {
            large: text_style(body_medium + 10.0, FontWeight::Regular),
            medium: text_style(body_medium + 8.0, FontWeight::Regular),
            small: text_style(body_medium + 6.0, FontWeight::Regular),
        },
        title: TypeScale {
            large: text_style(body_medium + 5.0, FontWeight::SemiBold),
            medium: text_style(body_medium + 2.0, FontWeight::SemiBold),
            small: text_style(body_medium, FontWeight::SemiBold),
        },
        body: TypeScale {
            large: text_style(body_medium + 2.0, FontWeight::Regular),
            medium: text_style(body_medium, FontWeight::Regular),
            small: text_style(body_medium - 2.0, FontWeight::Regular),
        },
        label: TypeScale {
            large: text_style(body_medium - 1.0, FontWeight::Medium),
            medium: text_style(body_medium - 2.0, FontWeight::Medium),
            small: text_style(body_medium - 3.0, FontWeight::Medium),
        },
    }
}

fn text_style(font_size: f32, font_weight: FontWeight) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 6.0,
        letter_spacing: 0.0,
        font_weight,
    }
}

/// A runtime-agnostic theme that can hold any of the three Design System
/// themes. This is the foundation for switching Design System at runtime:
/// every component stores its theme as `RwSignal<MultiTheme>` instead of a
/// concrete `*Theme`, and `MultiTheme` forwards the `Theme` trait to whichever
/// variant is active (today it is seeded from the compile-time feature, but a
/// running app can replace it via the global theme signal).
#[derive(Debug, Clone)]
pub struct MultiTheme {
    material: MaterialTheme,
    cupertino: CupertinoTheme,
    fluent: FluentTheme,
}

impl MultiTheme {
    /// Build a `MultiTheme` whose active variant matches the given Design
    /// System. Used when an app switches DS at runtime.
    pub fn new_for(ds: DesignSystem) -> Self {
        Self {
            material: MaterialTheme::new(),
            cupertino: CupertinoTheme::new(),
            fluent: FluentTheme::new(),
        }
        .activate(ds)
    }

    /// Record the active Design System. (The active variant is selected at read
    /// time via `current_design_system`, so this is a no-op placeholder kept for
    /// call-site clarity / future per-DS overrides.)
    pub fn activate(self, _ds: DesignSystem) -> Self {
        self
    }

    fn ds(&self) -> DesignSystem {
        current_design_system()
    }
}

impl Theme for MultiTheme {
    fn colors(&self) -> &ColorTokens {
        match self.ds() {
            DesignSystem::Material => self.material.colors(),
            DesignSystem::Cupertino => self.cupertino.colors(),
            DesignSystem::Fluent => self.fluent.colors(),
        }
    }
    fn spacing(&self) -> &SpacingTokens {
        match self.ds() {
            DesignSystem::Material => self.material.spacing(),
            DesignSystem::Cupertino => self.cupertino.spacing(),
            DesignSystem::Fluent => self.fluent.spacing(),
        }
    }
    fn typography(&self) -> &TypographyTokens {
        match self.ds() {
            DesignSystem::Material => self.material.typography(),
            DesignSystem::Cupertino => self.cupertino.typography(),
            DesignSystem::Fluent => self.fluent.typography(),
        }
    }
    fn radius(&self) -> &RadiusTokens {
        match self.ds() {
            DesignSystem::Material => self.material.radius(),
            DesignSystem::Cupertino => self.cupertino.radius(),
            DesignSystem::Fluent => self.fluent.radius(),
        }
    }
    fn shadow(&self) -> &ElevationTokens {
        match self.ds() {
            DesignSystem::Material => self.material.shadow(),
            DesignSystem::Cupertino => self.cupertino.shadow(),
            DesignSystem::Fluent => self.fluent.shadow(),
        }
    }
    fn motion(&self) -> &MotionTokens {
        match self.ds() {
            DesignSystem::Material => self.material.motion(),
            DesignSystem::Cupertino => self.cupertino.motion(),
            DesignSystem::Fluent => self.fluent.motion(),
        }
    }
    fn component(&self, id: ComponentId) -> ComponentTokens {
        match self.ds() {
            DesignSystem::Material => self.material.component(id),
            DesignSystem::Cupertino => self.cupertino.component(id),
            DesignSystem::Fluent => self.fluent.component(id),
        }
    }
}
