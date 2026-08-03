# Component Parity Roadmap

Status: working roadmap  
Last updated: 2026-08-03

This project targets a Floem component library with a Vuetify-like out-of-the-box developer experience across Material, Cupertino, and Fluent design systems.

Primary references:

- Vuetify All Components: https://vuetifyjs.com/en/components/all/
- Vuetify repository: https://github.com/vuetifyjs/vuetify
- Vuetify 0 repository: https://github.com/vuetifyjs/vuetify0

The Vuetify documentation page is the source of truth for component inventory. If this file and the official page diverge, update this file before implementing more components.

## Current implemented contract

Phase 1 has a cross-design-system implementation for these 10 components:

| Contract component | Material | Cupertino | Fluent | Current capability |
| --- | --- | --- | --- | --- |
| Button | `md-button` | `cupertino-button` | `fluent-button` | props, builder API, style resolution |
| Text | `md-text` | `cupertino-text` | `fluent-text` | props, builder API, style resolution |
| TextField | `md-text-field` | `cupertino-text-field` | `fluent-text-field` | props, builder API, style resolution |
| Checkbox | `md-checkbox` | `cupertino-checkbox` | `fluent-checkbox` | props, builder API, style resolution |
| Switch | `md-switch` | `cupertino-switch` | `fluent-switch` | props, builder API, style resolution |
| Dialog | `md-dialog` | `cupertino-dialog` | `fluent-dialog` | props, builder API, style resolution |
| Menu | `md-menu` | `cupertino-menu` | `fluent-menu` | props, builder API, style resolution |
| Card | `md-card` | `cupertino-card` | `fluent-card` | props, builder API, style resolution |
| Icon | `md-icon` | `cupertino-icon` | `fluent-icon` | props, builder API, style resolution |
| Divider | `md-divider` | `cupertino-divider` | `fluent-divider` | props, builder API, style resolution |

The Cupertino and Fluent crates for the implemented contract components resolve native Cupertino or Fluent theme tokens.

## Vuetify-aligned target inventory

The long-term target should track the Vuetify component surface area, grouped by capability so implementation can be staged without losing parity.

### Application and layout

- App
- Main
- Container
- Grid / Row / Col
- Spacer
- Divider
- Sheet
- Footer
- SystemBar
- Toolbar
- AppBar
- BottomNavigation
- NavigationDrawer
- Layout item registration / layout provider

### Actions and controls

- Button
- ButtonGroup
- FloatingActionButton
- Checkbox
- Radio
- RadioGroup
- Switch
- Slider
- RangeSlider
- Rating
- FileInput
- OTPInput

### Text entry and selection

- TextField
- Textarea
- Select
- Combobox
- Autocomplete
- Form
- Input validation messages

### Data display

- Text
- Icon
- Avatar
- Badge
- Chip
- List
- ListItem
- Card
- Table
- DataTable
- DataIterator
- Treeview
- Timeline
- Calendar
- Sparkline
- Kbd
- Code

### Navigation

- Breadcrumbs
- Tabs
- Window
- Stepper
- Pagination
- SlideGroup
- ItemGroup

### Feedback and overlays

- Alert
- Banner
- Dialog
- Menu
- Tooltip
- Snackbar
- Overlay
- ProgressCircular
- ProgressLinear
- SkeletonLoader
- EmptyState

### Media and motion

- Image
- Carousel
- Parallax
- InfiniteScroller
- VirtualScroller
- Hover
- ExpandTransition
- FadeTransition
- ScaleTransition
- SlideTransition

### Pickers

- DatePicker
- TimePicker
- ColorPicker

### Providers and services

- ThemeProvider
- LocaleProvider
- DisplayProvider
- DefaultsProvider
- IconsProvider
- DateAdapterProvider

## Vuetify 0 alignment notes

Vuetify 0 is treated as a capability reference for a lower-level, composable UI engine rather than only a styled component catalogue. For this Rust/Floem project that means:

- keep component contracts design-system-neutral;
- keep visual tokens outside user-facing component APIs;
- allow compile-time tree-shaking through per-component crates;
- keep state and interaction behavior testable without rendering;
- later add headless primitives where component behavior should be reusable across Material, Cupertino, and Fluent.

## Implementation order

1. Stabilize the current contract components across Material, Cupertino, and Fluent.
2. Add missing high-use Vuetify controls: `Select`, `RadioGroup`, `Slider`, `Textarea`, `Tooltip`, `Snackbar`, `Tabs`, `List`.
3. Add layout primitives: `App`, `Container`, `Grid`, `Divider`, `Sheet`, `Toolbar`, navigation shell components.
4. Add data-heavy components: `DataTable`, `Treeview`, `Timeline`, virtual scrolling.
5. Add providers and defaults system so app-level theme, locale, display, and icon settings can flow into every component.

## Acceptance criteria for a new component

Every new contract component must include:

- neutral props in `floem-core-theme/src/props/`;
- component tokens in `floem-core-theme/src/component_tokens/`;
- Material, Cupertino, and Fluent crates, even if non-Material starts as a skeleton;
- `floem-elements` feature re-exports with neutral names;
- tests for Material behavior and feature compilation for all design systems;
- no hard-coded visual values in public component contracts.
