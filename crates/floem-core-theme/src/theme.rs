use crate::component_tokens::material::MaterialComponentTokens;
use crate::component_tokens::{ComponentId, ComponentTokens};
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
        &self.colors
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
