use floem_animation::{AnimKind, AnimationScheduler, Clock, CompositorProp, FakeClock, Tween};

#[test]
fn tween_deterministic_with_fake_clock() {
    let mut clock = FakeClock::new();
    let mut sched = AnimationScheduler::new();
    let id = sched.start(
        AnimKind::Tween(Tween {
            from: 0.0,
            to: 100.0,
            duration_secs: 1.0,
            compositor: Some(CompositorProp::Opacity(0.0)),
        }),
        clock.now_secs(),
    );
    clock.advance(0.5);
    let samples = sched.tick(clock.now_secs());
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].id, id);
    assert!((samples[0].value - 50.0).abs() < 1e-6);
    assert_eq!(samples[0].compositor, Some(CompositorProp::Opacity(50.0)));
    assert!(!samples[0].finished);

    clock.advance(0.5);
    let samples = sched.tick(clock.now_secs());
    assert!(samples[0].finished);
    assert!(sched.is_empty());
}

#[test]
fn spring_and_keyframe() {
    let mut sched = AnimationScheduler::new();
    sched.start(
        AnimKind::Spring {
            from: 0.0,
            to: 1.0,
            stiffness: 200.0,
            damping: 20.0,
            compositor: Some(CompositorProp::Scale(0.0)),
        },
        0.0,
    );
    sched.start(
        AnimKind::Keyframe {
            keys: vec![(0.0, 0.0), (0.5, 10.0), (1.0, 0.0)],
            compositor: Some(CompositorProp::TranslateX(0.0)),
        },
        0.0,
    );
    let s = sched.tick(0.25);
    assert_eq!(s.len(), 2);
    let kf = s
        .iter()
        .find(|x| matches!(x.compositor, Some(CompositorProp::TranslateX(_))))
        .unwrap();
    assert!((kf.value - 5.0).abs() < 1e-6);
}

#[test]
fn compositor_only_transform_opacity_path() {
    let mut sched = AnimationScheduler::new();
    sched.start(
        AnimKind::Tween(Tween {
            from: 0.0,
            to: 1.0,
            duration_secs: 0.2,
            compositor: Some(CompositorProp::TranslateY(0.0)),
        }),
        0.0,
    );
    let s = sched.tick(0.1);
    assert!(matches!(
        s[0].compositor,
        Some(CompositorProp::TranslateY(_))
    ));
}
