#![deny(missing_docs)]
use crate::accessibility::{AccessAction, AccessRole};
use crate::{IntoView, View, ViewId, style_class, views::Decorators};

style_class!(
    /// The style class that is applied to buttons.
    pub ButtonClass
);

/// Creates a new simple [Button] view with the default [ButtonClass] style.
///
/// ### Examples
/// ```rust
/// # use floem::views::Button;
/// # use floem::prelude::{*, palette::css};
/// # use floem::views::Decorators;
/// # use floem::style::CursorStyle;
/// // Basic usage
/// let button1 = Button::new("Click me").action(move || println!("Button1 clicked!"));
/// let button2 = Button::new("Click me").action(move || println!("Button2 clicked!"));
/// // Apply styles for the button
/// let styled = Button::new("Click me")
///     .action(|| println!("Styled button clicked!"))
///     .style(|s| s
///         .border(1.0)
///         .border_radius(10.0)
///         .padding(10.0)
///         .background(css::YELLOW_GREEN)
///         .color(css::DARK_GREEN)
///         .cursor(CursorStyle::Pointer)
///         .active(|s| s.color(css::WHITE).background(css::RED))
///         .hover(|s| s.background(Color::from_rgb8(244, 67, 54)))
///         .focus_visible(|s| s.border(2.).border_color(css::BLUE))
///     );
/// ```
#[deprecated(since = "0.2.0", note = "Use Button::new() instead")]
pub fn button<V: IntoView + 'static>(child: V) -> Button {
    Button::new(child)
}

/// A simple Button view. See [`button`].
pub struct Button {
    id: ViewId,
}

impl View for Button {
    fn id(&self) -> ViewId {
        self.id
    }
}

impl Button {
    /// Creates a new [Button] with the given child view.
    ///
    /// ### Examples
    /// ```rust
    /// # use floem::views::Button;
    /// # use floem::prelude::{*, palette::css};
    /// # use floem::views::Decorators;
    /// # use floem::style::CursorStyle;
    /// // Basic usage
    /// let button1 = Button::new("Click me").action(move || println!("Button1 clicked!"));
    /// let button2 = Button::new("Click me").action(move || println!("Button2 clicked!"));
    /// // Apply styles for the button
    /// let styled = Button::new("Click me")
    ///     .action(|| println!("Styled button clicked!"))
    ///     .style(|s| s
    ///         .border(1.0)
    ///         .border_radius(10.0)
    ///         .padding(10.0)
    ///         .background(css::YELLOW_GREEN)
    ///         .color(css::DARK_GREEN)
    ///         .cursor(CursorStyle::Pointer)
    ///         .active(|s| s.color(css::WHITE).background(css::RED))
    ///         .hover(|s| s.background(Color::from_rgb8(244, 67, 54)))
    ///         .focus_visible(|s| s.border(2.).border_color(css::BLUE))
    ///     );
    /// ```
    ///
    /// ### Reactivity
    /// Button's label is not reactive. For reactive content, pass a reactive view as the child.
    pub fn new(child: impl IntoView) -> Self {
        let child = child.into_view();
        let id = ViewId::new();
        id.accessibility_role(AccessRole::Button);
        id.accessibility_actions([AccessAction::Default]);
        if let Some(text) = button_label_from_child(&child) {
            id.accessibility_label(text);
        }
        id.add_child(Box::new(child));
        Button { id }.class(ButtonClass)
    }
}

/// A trait that adds a `button` method to any type that implements `IntoView`.
pub trait ButtonExt {
    /// Create a [Button] from the parent.
    fn button(self) -> Button;
}
impl<T: IntoView + 'static> ButtonExt for T {
    fn button(self) -> Button {
        Button::new(self)
    }
}

fn button_label_from_child<V: View + 'static>(child: &V) -> Option<String> {
    use std::any::Any;
    (child as &dyn Any)
        .downcast_ref::<crate::views::Label>()
        .map(|label| label.text().to_string())
}
