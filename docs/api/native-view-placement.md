<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Native-view placement: snapping, mirroring, and cropping

The desktop native-view bridge has an OS-free half in
`floom_platform_desktop::native`: the translation from `NativeState` into a
frame the OS can be handed, plus the `NativeViewProvider` impl around it. The OS
half is a handful of calls behind `NativeViewHost`.

That split exists because:

- the placement arithmetic is testable without a window, which is where its bugs
  actually live;
- a second backend (Win32, X11) is a trait impl, not a copy of the file;
- what is left to device work is exactly the six methods on the trait.

## The origin flip belongs to the backend, and it is wrong first

`NativeState::rect` is top-left logical, in window coordinates. `ViewPlacement`
keeps that convention — top-left, snapped to whole device pixels — and does
**not** apply any OS convention. Win32 and X11 are top-left, so for them the
placement is the answer; AppKit's default origin is bottom-left, so the AppKit
backend converts with `os_frame`, and nothing else in the chain has to know
which of the two it is talking to.

Turning the placement into an OS frame is the part that is wrong first, and
wrong in a way that survives a demo. The conversion that suggests itself —
mirror in *logical* units, then snap to device pixels — is not the same function
as snapping and *then* mirroring, because snapping rounds each edge
independently. At 2x, a participant at logical `y 0.75..1.25` of a parent 10
units tall belongs on device row 2 (so `y_os = 17`), while mirroring first lands
it on row 3 mirrored to `y_os = 18`. One pixel, and it is one pixel *of
disagreement with the framework*, which is what a seam is: the same box painted
by the scene and placed by the OS must be the same pixels, or adjacent content
shows a hairline at some (not all) scales — the failure that is invisible on the
developer's 2x display and reported by one user at 1.5x.

So: **snap once, in `floom_native::to_device`, and mirror in device space.**
`os_frame` only ever receives a `DeviceRect`, which makes that structural rather
than a comment, and `mirror_device` is an involution, which makes the invariant
checkable — mirroring an OS frame back must reproduce the very rect the
framework snapped.

Two halves of the same bug are worth stating separately:

- the height to mirror against is the **parent view's**, not the window's. They
  are the same number for a participant added straight to the content view and
  differ the moment one goes into a container, which is precisely the case
  nobody has on day one;
- whether a flip is needed at all is a property of the *specific parent view*,
  not of the platform: `NSView::isFlipped` is per-view, and a window whose
  content view is already flipped wants top-left. The AppKit backend asks the
  view rather than assuming.

## A crop is a container, for backends that crop with their own frames

`NativeState` separates placement from clipping, so a clipped participant keeps
its size and is shown through a window. A backend that crops by clipping a
subview — AppKit, and the `Platform` arm of `ClipSupport` generally — needs that
window to be a *view*: the participant goes inside a container the size of the
crop and is offset within it, which is what `split_placement` computes.
Shrinking the participant's own frame instead would rescale its content, and
scrolling it by moving its bounds origin would move the participant's own scroll
position — a crop of somebody else's view has to be somebody else's view.
