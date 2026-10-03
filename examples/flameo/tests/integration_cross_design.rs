use floom_core_theme::component_tokens::ComponentId;
use floom_core_theme::props::button::ButtonSize;
use floom_core_theme::theme::{CupertinoTheme, FluentTheme, MaterialTheme, Theme};

#[test]
fn three_design_systems_implement_theme_trait() {
    fn assert_is_theme<T: Theme>(t: &T) -> &T {
        t
    }

    let mat = MaterialTheme::new();
    let cup = CupertinoTheme::new();
    let flu = FluentTheme::new();

    let material = assert_is_theme(&mat);
    let cupertino = assert_is_theme(&cup);
    let fluent = assert_is_theme(&flu);

    assert!(material.colors().primary.b > 0);
    assert!(cupertino.colors().primary.g > 0);
    assert!(fluent.colors().primary.b > 0);
}

#[test]
fn each_design_system_has_distinct_primary_colors() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    assert_eq!(material.colors().primary.r, 103);
    assert_eq!(material.colors().primary.g, 80);
    assert_eq!(material.colors().primary.b, 164);

    assert_eq!(cupertino.colors().primary.r, 0);
    assert_eq!(cupertino.colors().primary.g, 122);
    assert_eq!(cupertino.colors().primary.b, 255);

    assert_eq!(fluent.colors().primary.r, 0);
    assert_eq!(fluent.colors().primary.g, 120);
    assert_eq!(fluent.colors().primary.b, 212);

    assert_ne!(material.colors().primary, cupertino.colors().primary);
    assert_ne!(material.colors().primary, fluent.colors().primary);
    assert_ne!(cupertino.colors().primary, fluent.colors().primary);
}

#[test]
fn each_design_system_has_distinct_radius_tokens() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    assert_eq!(material.radius().md, 12.0);
    assert_eq!(cupertino.radius().md, 10.0);
    assert_eq!(fluent.radius().md, 4.0);
}

#[test]
fn each_design_system_has_distinct_button_tokens() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    use floom_core_theme::component_tokens::ComponentTokens;

    let ComponentTokens::Button(mb) = material.component(ComponentId::Button) else {
        panic!("expected Button");
    };
    let ComponentTokens::Button(cb) = cupertino.component(ComponentId::Button) else {
        panic!("expected Button");
    };
    let ComponentTokens::Button(fb) = fluent.component(ComponentId::Button) else {
        panic!("expected Button");
    };

    assert_eq!(mb.height, 40.0);
    assert_eq!(cb.height, 44.0);
    assert_eq!(fb.height, 32.0);
}

#[test]
fn each_design_system_has_distinct_input_tokens() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    use floom_core_theme::component_tokens::ComponentTokens;

    let ComponentTokens::TextField(mf) = material.component(ComponentId::TextField) else {
        panic!("expected TextField");
    };
    let ComponentTokens::TextField(cf) = cupertino.component(ComponentId::TextField) else {
        panic!("expected TextField");
    };
    let ComponentTokens::TextField(ff) = fluent.component(ComponentId::TextField) else {
        panic!("expected TextField");
    };

    assert_eq!(mf.height, 48.0);
    assert_eq!(cf.height, 44.0);
    assert_eq!(ff.height, 32.0);
}

#[test]
fn typography_differs_per_design_system() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    let m_body = material.typography().body.medium.font_size;
    let c_body = cupertino.typography().body.medium.font_size;
    let f_body = fluent.typography().body.medium.font_size;

    assert_eq!(m_body, 14.0);
    assert_eq!(c_body, 17.0);
    assert_eq!(f_body, 14.0);
}

#[test]
fn all_themes_are_clone() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    let m2 = material.clone();
    let c2 = cupertino.clone();
    let f2 = fluent.clone();

    assert_eq!(m2.colors().primary, material.colors().primary);
    assert_eq!(c2.colors().primary, cupertino.colors().primary);
    assert_eq!(f2.colors().primary, fluent.colors().primary);
}

#[test]
fn motion_tokens_present_and_nonzero() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    for m in [material.motion(), cupertino.motion(), fluent.motion()] {
        assert!(m.duration_short > 0.0);
        assert!(m.duration_medium > 0.0);
        assert!(m.duration_long > 0.0);
    }
}

#[test]
fn spacing_tokens_monotonic() {
    fn assert_monotonic(sp: &floom_core_theme::tokens::spacing::SpacingTokens) {
        assert!(sp.xs < sp.sm);
        assert!(sp.sm < sp.md);
        assert!(sp.md < sp.lg);
        assert!(sp.lg < sp.xl);
        assert!(sp.xl < sp.xxl);
    }

    assert_monotonic(MaterialTheme::new().spacing());
    assert_monotonic(CupertinoTheme::new().spacing());
    assert_monotonic(FluentTheme::new().spacing());
}

#[test]
fn shadow_tokens_have_six_levels() {
    let material = MaterialTheme::new();
    assert_eq!(material.shadow().levels.len(), 6);
}

#[test]
fn button_height_per_design_system_via_elements_prelude() {
    let btn_m = floom_elements::button::Button::new("M")
        .size(ButtonSize::Medium)
        .with_theme(MaterialTheme::new());
    assert_eq!(btn_m.resolved_style().height, 40.0);

    let m_small = floom_elements::button::Button::new("S")
        .size(ButtonSize::Small)
        .with_theme(MaterialTheme::new());
    assert!(m_small.resolved_style().height <= 40.0);

    let m_large = floom_elements::button::Button::new("L")
        .size(ButtonSize::Large)
        .with_theme(MaterialTheme::new());
    assert!(m_large.resolved_style().height >= 48.0);
}
