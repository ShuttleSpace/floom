//! Event routing: hit-testing, capture/bubble phases, pointer capture.

use std::collections::HashMap;

use floem_types::{ElementId, NativeViewId, Point, Rect};

/// Dispatch phase for a routed pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// A single stop on the route from root → target (or reverse for bubble).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteStop {
    pub id: ElementId,
    pub phase: EventPhase,
}

/// Hit region registered for an element (logical bounds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitRegion {
    pub bounds: Rect,
    /// When set, native compositor participants win over sibling Floem hits.
    pub native: Option<NativeViewId>,
    pub z: i32,
}

/// Hit-test result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub target: ElementId,
    pub native: Option<NativeViewId>,
}

/// Spatial hit tester over registered regions (front-most z wins).
#[derive(Debug, Default)]
pub struct HitTester {
    regions: HashMap<ElementId, HitRegion>,
}

impl HitTester {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_region(&mut self, id: ElementId, region: HitRegion) {
        self.regions.insert(id, region);
    }

    pub fn remove(&mut self, id: ElementId) {
        self.regions.remove(&id);
    }

    pub fn hit_test(&self, point: Point) -> Option<Hit> {
        let mut best: Option<(ElementId, HitRegion)> = None;
        for (&id, region) in &self.regions {
            if !region.bounds.contains(point) {
                continue;
            }
            match best {
                None => best = Some((id, *region)),
                Some((_, prev)) => {
                    // Native regions beat same-z non-native; then higher z.
                    let native_boost = |r: &HitRegion| if r.native.is_some() { 1 } else { 0 };
                    let better = region.z > prev.z
                        || (region.z == prev.z && native_boost(region) > native_boost(&prev));
                    if better {
                        best = Some((id, *region));
                    }
                }
            }
        }
        best.map(|(target, r)| Hit {
            target,
            native: r.native,
        })
    }
}

/// Pointer capture state (exclusive target until released).
#[derive(Debug, Default)]
pub struct PointerCapture {
    captured: Option<ElementId>,
}

impl PointerCapture {
    pub fn capture(&mut self, id: ElementId) {
        self.captured = Some(id);
    }

    pub fn release(&mut self) {
        self.captured = None;
    }

    pub fn get(&self) -> Option<ElementId> {
        self.captured
    }
}

/// Builds capture → target → bubble routes given parent links.
#[derive(Debug, Default)]
pub struct EventRouter {
    /// child → parent
    parents: HashMap<ElementId, ElementId>,
    pub capture: PointerCapture,
    pub hits: HitTester,
}

impl EventRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_parent(&mut self, child: ElementId, parent: ElementId) {
        self.parents.insert(child, parent);
    }

    pub fn ancestors(&self, id: ElementId) -> Vec<ElementId> {
        let mut out = Vec::new();
        let mut cur = id;
        while let Some(&p) = self.parents.get(&cur) {
            out.push(p);
            cur = p;
        }
        out
    }

    /// Resolve the effective target (capture overrides hit-test).
    pub fn resolve_target(&self, point: Point) -> Option<Hit> {
        if let Some(id) = self.capture.get() {
            let native = self.hits.regions.get(&id).and_then(|r| r.native);
            return Some(Hit {
                target: id,
                native,
            });
        }
        self.hits.hit_test(point)
    }

    /// Full route: root…parent (Capture) → target (Target) → parent…root (Bubble).
    pub fn route(&self, target: ElementId) -> Vec<RouteStop> {
        let mut ancestors = self.ancestors(target);
        ancestors.reverse(); // root → … → parent
        let mut stops = Vec::with_capacity(ancestors.len() * 2 + 1);
        for id in &ancestors {
            stops.push(RouteStop {
                id: *id,
                phase: EventPhase::Capture,
            });
        }
        stops.push(RouteStop {
            id: target,
            phase: EventPhase::Target,
        });
        for id in ancestors.iter().rev() {
            stops.push(RouteStop {
                id: *id,
                phase: EventPhase::Bubble,
            });
        }
        stops
    }

    /// Hit-test + route in one step.
    pub fn dispatch_route(&self, point: Point) -> Option<(Hit, Vec<RouteStop>)> {
        let hit = self.resolve_target(point)?;
        let route = self.route(hit.target);
        Some((hit, route))
    }
}
