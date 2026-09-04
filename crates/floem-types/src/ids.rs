//! Generational identity handles.

/// Compact generational ID: low 32 bits index, high 32 bits generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationalId {
    raw: u64,
}

impl GenerationalId {
    pub const INVALID: Self = Self { raw: 0 };

    #[inline]
    pub const fn new(index: u32, generation: u32) -> Self {
        // Reserve (0, 0) as invalid / never allocated.
        Self {
            raw: ((generation as u64) << 32) | (index as u64),
        }
    }

    #[inline]
    pub const fn index(self) -> u32 {
        self.raw as u32
    }

    #[inline]
    pub const fn generation(self) -> u32 {
        (self.raw >> 32) as u32
    }

    #[inline]
    pub const fn is_invalid(self) -> bool {
        self.raw == 0
    }

    #[inline]
    pub const fn raw(self) -> u64 {
        self.raw
    }

    /// Returns true when `self` matches the live generation for `index`.
    #[inline]
    pub const fn matches_generation(self, live_generation: u32) -> bool {
        !self.is_invalid() && self.generation() == live_generation
    }
}

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(GenerationalId);

        impl $name {
            pub const INVALID: Self = Self(GenerationalId::INVALID);

            #[inline]
            pub const fn new(index: u32, generation: u32) -> Self {
                Self(GenerationalId::new(index, generation))
            }

            #[inline]
            pub const fn index(self) -> u32 {
                self.0.index()
            }

            #[inline]
            pub const fn generation(self) -> u32 {
                self.0.generation()
            }

            #[inline]
            pub const fn is_invalid(self) -> bool {
                self.0.is_invalid()
            }

            #[inline]
            pub const fn matches_generation(self, live_generation: u32) -> bool {
                self.0.matches_generation(live_generation)
            }

            #[inline]
            pub const fn raw(self) -> u64 {
                self.0.raw()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::INVALID
            }
        }
    };
}

define_id!(
    /// Persistent element identity in the retained runtime.
    ElementId
);
define_id!(
    /// Node identity in the retained scene graph.
    SceneNodeId
);
define_id!(
    /// Shared GPU/CPU resource identity (images, glyphs, etc.).
    ResourceId
);
define_id!(
    /// Framework window identity (not an OS handle).
    WindowId
);
define_id!(
    /// Render surface identity.
    SurfaceId
);
define_id!(
    /// Native compositor participant (WebView, video, …).
    NativeViewId
);
define_id!(
    /// Capability-oriented platform backend identity.
    PlatformId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_is_zero() {
        assert!(ElementId::INVALID.is_invalid());
        assert_eq!(ElementId::default(), ElementId::INVALID);
    }

    #[test]
    fn generation_roundtrip() {
        let id = ElementId::new(7, 3);
        assert_eq!(id.index(), 7);
        assert_eq!(id.generation(), 3);
        assert!(id.matches_generation(3));
        assert!(!id.matches_generation(4));
    }

    #[test]
    fn stale_generation_rejected() {
        let live = ElementId::new(1, 2);
        let stale = ElementId::new(1, 1);
        assert!(live.matches_generation(2));
        assert!(!stale.matches_generation(2));
        assert_ne!(live, stale);
    }

    #[test]
    fn id_kinds_are_distinct_types() {
        let e = ElementId::new(1, 1);
        let s = SceneNodeId::new(1, 1);
        assert_eq!(e.raw(), s.raw());
    }
}
