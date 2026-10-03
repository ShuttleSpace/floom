//! Theme tokens: color, typography, spacing, radius, elevation, motion.

use floem_types::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorTokens {
    pub background: Color,
    pub surface: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub danger: Color,
    pub text: Color,
    pub text_muted: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    pub font_family: String,
    pub size_sm: f32,
    pub size_md: f32,
    pub size_lg: f32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusTokens {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub full: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationTokens {
    pub level0: f32,
    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTokens {
    pub duration_fast_ms: u32,
    pub duration_normal_ms: u32,
    pub duration_slow_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub scheme: ColorScheme,
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub radius: RadiusTokens,
    pub elevation: ElevationTokens,
    pub motion: MotionTokens,
    /// Platform adaptive density scale (1.0 desktop, >1 touch).
    pub density: f32,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            scheme: ColorScheme::Light,
            colors: ColorTokens {
                background: Color::rgba(250, 250, 250, 255),
                surface: Color::rgba(255, 255, 255, 255),
                primary: Color::rgba(37, 99, 235, 255),
                on_primary: Color::rgba(255, 255, 255, 255),
                danger: Color::rgba(220, 38, 38, 255),
                text: Color::rgba(17, 24, 39, 255),
                text_muted: Color::rgba(107, 114, 128, 255),
            },
            typography: TypographyTokens {
                font_family: "sans-serif".into(),
                size_sm: 12.0,
                size_md: 14.0,
                size_lg: 18.0,
                line_height: 1.4,
            },
            spacing: SpacingTokens {
                xs: 4.0,
                sm: 8.0,
                md: 16.0,
                lg: 24.0,
                xl: 32.0,
            },
            radius: RadiusTokens {
                sm: 4.0,
                md: 8.0,
                lg: 12.0,
                full: 9999.0,
            },
            elevation: ElevationTokens {
                level0: 0.0,
                level1: 2.0,
                level2: 6.0,
                level3: 12.0,
            },
            motion: MotionTokens {
                duration_fast_ms: 100,
                duration_normal_ms: 200,
                duration_slow_ms: 350,
            },
            density: 1.0,
        }
    }

    pub fn dark() -> Self {
        let mut t = Self::light();
        t.scheme = ColorScheme::Dark;
        t.colors = ColorTokens {
            background: Color::rgba(17, 24, 39, 255),
            surface: Color::rgba(31, 41, 55, 255),
            primary: Color::rgba(96, 165, 250, 255),
            on_primary: Color::rgba(17, 24, 39, 255),
            danger: Color::rgba(248, 113, 113, 255),
            text: Color::rgba(243, 244, 246, 255),
            text_muted: Color::rgba(156, 163, 175, 255),
        };
        t
    }

    /// Platform-adaptive hook (touch targets / density).
    pub fn with_platform_density(mut self, density: f32) -> Self {
        self.density = density;
        self.spacing.xs *= density;
        self.spacing.sm *= density;
        self.spacing.md *= density;
        self.spacing.lg *= density;
        self.spacing.xl *= density;
        self
    }
}

/// Propagates the active theme to interested listeners (test/simple).
#[derive(Debug, Default)]
pub struct ThemeScope {
    theme: Option<Theme>,
    generation: u64,
}

impl ThemeScope {
    pub fn set(&mut self, theme: Theme) {
        self.theme = Some(theme);
        self.generation += 1;
    }

    pub fn get(&self) -> Option<&Theme> {
        self.theme.as_ref()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}
