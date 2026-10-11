use floom_theme::{ColorScheme, Theme, ThemeScope};

#[test]
fn light_and_dark_tokens() {
    let light = Theme::light();
    let dark = Theme::dark();
    assert_eq!(light.scheme, ColorScheme::Light);
    assert_eq!(dark.scheme, ColorScheme::Dark);
    assert_ne!(light.colors.background, dark.colors.background);
    assert!(light.spacing.md > 0.0);
    assert!(light.motion.duration_normal_ms > 0);
    assert!(light.elevation.level2 > light.elevation.level1);
    assert!(light.radius.md > 0.0);
    assert!(!light.typography.font_family.is_empty());
}

#[test]
fn theme_propagation_and_density() {
    let mut scope = ThemeScope::default();
    scope.set(Theme::light().with_platform_density(1.5));
    assert_eq!(scope.generation(), 1);
    let t = scope.get().unwrap();
    assert!((t.density - 1.5).abs() < f32::EPSILON);
    assert!((t.spacing.md - 24.0).abs() < 0.01); // 16 * 1.5
}
