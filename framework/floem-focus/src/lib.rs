//! Focus scopes and keyboard traversal (Tab / Shift-Tab).

use floem_types::ElementId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusScopeId(pub u32);

#[derive(Debug, Clone)]
struct Scope {
    #[allow(dead_code)]
    id: FocusScopeId,
    /// Focusable elements in traversal order.
    order: Vec<ElementId>,
    /// Nested scopes (visited after exhausting current order when wrapping).
    children: Vec<FocusScopeId>,
}

/// Manages focused element and platform focus synchronization hooks.
#[derive(Debug, Default)]
pub struct FocusManager {
    scopes: Vec<Scope>,
    active_scope: Option<FocusScopeId>,
    focused: Option<ElementId>,
    /// Last platform focus notification (for tests / adapters).
    pub platform_focused: Option<ElementId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_scope(&mut self, parent: Option<FocusScopeId>) -> FocusScopeId {
        let id = FocusScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope {
            id,
            order: Vec::new(),
            children: Vec::new(),
        });
        if let Some(p) = parent {
            self.scopes[p.0 as usize].children.push(id);
        } else if self.active_scope.is_none() {
            self.active_scope = Some(id);
        }
        id
    }

    pub fn register(&mut self, scope: FocusScopeId, id: ElementId) {
        let order = &mut self.scopes[scope.0 as usize].order;
        if !order.contains(&id) {
            order.push(id);
        }
    }

    /// Drops `id` from traversal order in every scope.
    ///
    /// Unmounting a widget without this leaves the traversal order holding ids
    /// that no longer exist, so Tab would focus nothing and lose its place.
    pub fn unregister(&mut self, id: ElementId) {
        for scope in &mut self.scopes {
            scope.order.retain(|existing| *existing != id);
        }
        if self.focused == Some(id) {
            self.focused = None;
            self.platform_focused = None;
        }
    }

    /// Whether `id` is still in the traversal order of any scope.
    pub fn is_registered(&self, id: ElementId) -> bool {
        self.scopes.iter().any(|scope| scope.order.contains(&id))
    }

    pub fn focused(&self) -> Option<ElementId> {
        self.focused
    }

    pub fn active_scope(&self) -> Option<FocusScopeId> {
        self.active_scope
    }

    pub fn set_active_scope(&mut self, scope: FocusScopeId) {
        self.active_scope = Some(scope);
    }

    /// Programmatic focus + platform hook.
    pub fn focus(&mut self, id: ElementId) {
        self.focused = Some(id);
        self.platform_focused = Some(id);
    }

    pub fn blur(&mut self) {
        self.focused = None;
        self.platform_focused = None;
    }

    /// Platform reports window focus change.
    pub fn on_platform_focus(&mut self, focused: bool) {
        if !focused {
            self.platform_focused = None;
        } else if let Some(id) = self.focused {
            self.platform_focused = Some(id);
        }
    }

    pub fn traverse(&mut self, dir: FocusDirection) -> Option<ElementId> {
        let scope_id = self.active_scope?;
        let order = &self.scopes[scope_id.0 as usize].order;
        if order.is_empty() {
            return None;
        }
        let next = match self.focused {
            None => match dir {
                FocusDirection::Forward => order[0],
                FocusDirection::Backward => *order.last().unwrap(),
            },
            Some(cur) => {
                let idx = order.iter().position(|e| *e == cur).unwrap_or(0);
                match dir {
                    FocusDirection::Forward => order[(idx + 1) % order.len()],
                    FocusDirection::Backward => {
                        if idx == 0 {
                            order[order.len() - 1]
                        } else {
                            order[idx - 1]
                        }
                    }
                }
            }
        };
        self.focus(next);
        Some(next)
    }

    pub fn tab(&mut self) -> Option<ElementId> {
        self.traverse(FocusDirection::Forward)
    }

    pub fn shift_tab(&mut self) -> Option<ElementId> {
        self.traverse(FocusDirection::Backward)
    }
}
