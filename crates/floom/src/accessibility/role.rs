//! Accessible role taxonomy.
//!
//! Mirrors the WAI-ARIA role set and the native role vocabularies of the
//! platforms Floem targets:
//!
//! - macOS / iOS: `NSAccessibility` roles
//! - Android: `AccessibilityNodeInfo` role flags
//! - Linux: AT-SPI2 roles
//! - Windows: UI Automation control types
//!
//! The enum is deliberately platform-neutral; each backend maps it to its
//! native role vocabulary when it builds the native a11y tree.

/// A platform-neutral accessibility role.
///
/// Start from [`AccessRole::Unknown`] and set the specific role when a view
/// knows what it represents (a `button` view → [`AccessRole::Button`], etc.).
/// Unknown/unspecified nodes are typically skipped or merged by backends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccessRole {
    /// No specific role is known.
    Unknown,
    /// A node that should be ignored by the accessibility tree (pure decoration).
    None,
    // --- Generic containers ------------------------------------------------
    /// A top-level application window.
    Window,
    /// The application root.
    Application,
    /// A dialog / modal sheet.
    Dialog,
    /// A logical grouping of other nodes.
    Group,
    /// A visual panel / region.
    Panel,
    /// A separator / divider.
    Divider,
    // --- Text & static content --------------------------------------------
    /// Generic static text or label.
    Text,
    /// A heading / title.
    Heading,
    /// A paragraph block.
    Paragraph,
    /// A hyperlink.
    Link,
    /// A non-interactive image.
    Image,
    // --- Interactive widgets ----------------------------------------------
    /// A push button.
    Button,
    /// A checkbox (tri-state capable via [`crate::accessibility::AccessState`]).
    CheckBox,
    /// A radio button.
    RadioButton,
    /// An on/off switch.
    Switch,
    /// A slider (continuous value).
    Slider,
    /// A spin button (discrete value).
    SpinButton,
    /// A determinate progress indicator.
    ProgressBar,
    // --- Editable ----------------------------------------------------------
    /// A single-line text field.
    TextField,
    /// A search field.
    SearchField,
    // --- List / grid -------------------------------------------------------
    /// A list container.
    List,
    /// A list item.
    ListItem,
    /// A grid / table.
    Grid,
    /// A grid cell.
    Cell,
    // --- Tabs --------------------------------------------------------------
    /// A single tab.
    Tab,
    /// A tab strip / tab list.
    TabList,
    /// The content panel for a tab.
    TabPanel,
    // --- Menus -------------------------------------------------------------
    /// A menu.
    Menu,
    /// A menu bar.
    MenuBar,
    /// A menu item.
    MenuItem,
    // --- Scrolling ---------------------------------------------------------
    /// A scrollable viewport.
    ScrollView,
    /// A scroll bar.
    ScrollBar,
    // --- Alerts & hints ----------------------------------------------------
    /// An alert / error message.
    Alert,
    /// A status / live region.
    Status,
    /// A tooltip.
    Tooltip,
}

impl Default for AccessRole {
    fn default() -> Self {
        AccessRole::Unknown
    }
}
