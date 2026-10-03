//! Lifecycle and reuse tests for the native view pool.

use floom_native::NativeViewPool;

#[test]
fn acquire_returns_same_slot_for_same_key() {
    let mut pool: NativeViewPool<usize> = NativeViewPool::new();
    let a = pool.acquire(1);
    let b = pool.acquire(1);
    assert_eq!(a, b);
    assert_eq!(pool.live_count(), 1);
}

#[test]
fn release_then_acquire_other_key_recycles_slot() {
    let mut pool: NativeViewPool<usize> = NativeViewPool::new();
    let s1 = pool.acquire(1);
    assert_eq!(pool.release(&1), Some(s1));
    assert_eq!(pool.live_count(), 0);
    assert_eq!(pool.free_count(), 1);
    let s2 = pool.acquire(2);
    assert_eq!(s2.id, s1.id, "freed slot should be reused for a new key");
    assert_eq!(
        s2.generation,
        s1.generation + 1,
        "a recycled slot must be distinguishable from the one whose handle was destroyed"
    );
    assert_eq!(pool.live_count(), 1);
    assert_eq!(pool.free_count(), 0);
}

#[test]
fn bump_generation_changes_live_slot_only() {
    let mut pool: NativeViewPool<usize> = NativeViewPool::new();
    let s1 = pool.acquire(7);
    let bumped = pool.bump_generation(&7).unwrap();
    assert_eq!(bumped.id, s1.id);
    assert_eq!(bumped.generation, s1.generation + 1);
    // Recycling bumps again on top of the manual bump: every rebind gets a
    // generation no previous holder of this id has seen.
    pool.release(&7);
    let s2 = pool.acquire(7);
    assert_eq!(s2.id, bumped.id);
    assert_eq!(s2.generation, bumped.generation + 1);
}

#[test]
fn every_recycle_of_the_same_id_gets_a_fresh_generation() {
    // The 16.2 case: scrolling 1k rows through a 10-row viewport reuses the
    // same handful of ids over and over. A backend told "slot id 3, generation
    // 1" twice cannot tell the second row from the first, and would keep
    // drawing the destroyed view.
    let mut pool: NativeViewPool<usize> = NativeViewPool::new();
    let mut ids = Vec::new();
    let mut generations = Vec::new();
    for key in 0..50 {
        let slot = pool.acquire(key);
        ids.push(slot.id);
        generations.push(slot.generation);
        pool.release(&key);
    }
    assert!(
        ids.windows(2).all(|w| w[0] == w[1]),
        "with one row live at a time every acquire should reuse one id"
    );
    assert_eq!(
        generations,
        (1..=50u32).collect::<Vec<_>>(),
        "each recycle must bump the generation"
    );
}

#[test]
fn pool_stays_bounded_under_churn() {
    let mut pool: NativeViewPool<u32> = NativeViewPool::new();
    for k in 0..10_000u32 {
        pool.acquire(k);
    }
    assert_eq!(pool.live_count(), 10_000);
    for k in 0..10_000u32 {
        pool.release(&k);
    }
    assert_eq!(pool.live_count(), 0);
    assert_eq!(pool.free_count(), 10_000);
    // Re-acquire 100 distinct keys reuses freed slots; count stays bounded.
    for k in 0..100u32 {
        pool.acquire(1_000_000 + k);
    }
    assert_eq!(pool.live_count(), 100);
    assert!(pool.free_count() >= 9_900);
}

#[test]
fn acquire_unknown_key_returns_none_on_release() {
    let mut pool: NativeViewPool<u32> = NativeViewPool::new();
    pool.acquire(42);
    assert_eq!(pool.release(&99), None);
}
