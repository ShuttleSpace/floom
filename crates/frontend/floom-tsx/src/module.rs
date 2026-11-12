// SPDX-License-Identifier: MIT
//! The `"floom"` module's exports: the vocabulary a TSX author writes against.
//!
//! One list, read by the compiler that lowers a TSX file and by the generator
//! that writes the `.d.ts`. A name the runtime does not implement cannot be
//! declared, and a name the runtime implements cannot go undeclared: the enum
//! *is* the list, and both sides walk it.
//!
//! The rule this replaces was "the declaration is a string constant and the
//! runtime is a set of string literals in a `match`". Those two lists drift:
//! a runtime that grows an export leaves the editor offering nothing, and a
//! declaration that grows an export offers the editor a function that does
//! not exist. Neither is visible until a person writes the app.

/// One export of the `"floom"` module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleExport {
    /// `const [value, setValue] = createSignal(initial)`.
    CreateSignal,
    /// `const value = createMemo(() => …)`.
    CreateMemo,
    /// `createEffect(() => …)` — a binding-less reactive statement.
    CreateEffect,
    /// `provideContext(name, value)` — publishes a value to descendants.
    ProvideContext,
    /// `useContext(name)` — reads a value a descendant was provided.
    UseContext,
    /// `Component` — a function that returns JSX.
    Component,
    /// `mount(App)` — names the root component.
    Mount,
    /// `app.setName(name)` — the application's own name.
    App,
    /// `window.setTitle(..)` and the rest of the window surface.
    Window,
    /// `platform.require(capability, policy)`.
    Platform,
    /// `Capability` — what a platform backend can be asked for.
    Capability,
    /// `Policy` — what happens when a capability is missing.
    Policy,
}

impl ModuleExport {
    /// Every export, in the order the declaration lists them.
    pub const ALL: &'static [ModuleExport] = &[
        ModuleExport::CreateSignal,
        ModuleExport::CreateMemo,
        ModuleExport::CreateEffect,
        ModuleExport::ProvideContext,
        ModuleExport::UseContext,
        ModuleExport::Component,
        ModuleExport::Mount,
        ModuleExport::App,
        ModuleExport::Window,
        ModuleExport::Platform,
        ModuleExport::Capability,
        ModuleExport::Policy,
    ];

    /// The name the module exports it under.
    pub fn name(self) -> &'static str {
        match self {
            ModuleExport::CreateSignal => "createSignal",
            ModuleExport::CreateMemo => "createMemo",
            ModuleExport::CreateEffect => "createEffect",
            ModuleExport::ProvideContext => "provideContext",
            ModuleExport::UseContext => "useContext",
            ModuleExport::Component => "Component",
            ModuleExport::Mount => "mount",
            ModuleExport::App => "app",
            ModuleExport::Window => "window",
            ModuleExport::Platform => "platform",
            ModuleExport::Capability => "Capability",
            ModuleExport::Policy => "Policy",
        }
    }

    /// The export a name refers to, if the module has one.
    pub fn from_name(name: &str) -> Option<Self> {
        ModuleExport::ALL
            .iter()
            .copied()
            .find(|export| export.name() == name)
    }

    /// Whether the module exports something under `name`.
    pub fn is_export(name: &str) -> bool {
        ModuleExport::from_name(name).is_some()
    }

    /// The TypeScript declaration of this export, as it appears inside
    /// `declare module "floom"`.
    pub fn declaration(self) -> &'static str {
        match self {
            ModuleExport::CreateSignal => {
                "export function createSignal<T>(value: T): [Floom.Accessor<T>, Floom.Setter<T>];"
            }
            ModuleExport::CreateMemo => {
                "export function createMemo<T>(fn: () => T): Floom.Accessor<T>;"
            }
            ModuleExport::CreateEffect => "export function createEffect(fn: () => void): void;",
            ModuleExport::ProvideContext => {
                "export function provideContext<T>(name: string, value: T): void;"
            }
            ModuleExport::UseContext => "export function useContext<T = string>(name: string): T;",
            ModuleExport::Component => "export type Component = Floom.Component;",
            ModuleExport::Mount => "export function mount(component: Component): void;",
            ModuleExport::App => "export const app: { setName(name: string): void };",
            ModuleExport::Window => {
                "export const window: {\n    setTitle(title: string): void;\n    \
                 setSize(width: number, height: number): void;\n    \
                 setSize(size: string): void;\n    \
                 setResizable(resizable: boolean): void;\n  };"
            }
            ModuleExport::Platform => {
                "export const platform: {\n    /** @param capability windowing | clipboard | ime | accessibility | webgpu | software_render | touch | native_view */\n    \
                 require(capability: Capability, onUnsupported: Policy): void;\n  };"
            }
            ModuleExport::Capability => {
                "export type Capability = \"windowing\" | \"clipboard\" | \"ime\" | \"accessibility\" | \"webgpu\" | \"software_render\" | \"touch\" | \"native_view\";"
            }
            ModuleExport::Policy => "export type Policy = \"warn\" | \"fail\" | \"exit\";",
        }
    }
}
