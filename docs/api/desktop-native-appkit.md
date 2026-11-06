<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# The AppKit native-view host

A desktop participant on macOS is a real `NSView` added to the window's content
view, and `floom_platform_desktop::native_appkit` is the `NativeViewHost` that
puts it there. It is deliberately small: everything with arithmetic in it — the
snap, the origin conversion, the crop split, the conversion to points — lives in
`floom_platform_desktop::native` and is tested without a window. What is left is
the part that genuinely needs an OS.

See `docs/api/native-view-placement.md` for the arithmetic.

## Two views per participant, on purpose

Each participant is a **container** holding the participant's own view. The
container is the crop: it is sized to the visible window onto the participant,
and the participant is offset inside it, so AppKit's ordinary "a view clips its
subviews to its own bounds" does the cropping. That is why this backend declares
`ClipSupport::Platform` honestly rather than degrading every clipped
participant — the platform really does crop here.

Shrinking the participant's own frame would rescale its content, and moving its
`bounds` origin would scroll it. A crop of somebody else's view has to be
somebody else's view.

The container is created for *every* participant, crop or no crop, so that the
cropping path is the only path. The cost is one extra `NSView` per participant,
which is cheap next to a list that is already reasoning about how many of them
exist; the alternative is a second arrangement that only a clipped participant
reaches.

## The container is flipped, and the parent is asked about itself

The container overrides `isFlipped` to `true`, so the coordinates it reads for
the participant inside it are top-left — the framework's own convention, with no
conversion at all. The container's *own* frame is expressed in its parent's
convention, which is not ours to decide: `NSView::isFlipped` is per-view, and
winit's content view is flipped while a bare `NSView` is not. So the backend asks
the view (`parent.isFlipped()`) instead of assuming, and `os_frame` turns that
answer into either a copy or a mirror.

## What this backend declares, and what it does not

- `clip: Platform` — the clipping container above.
- `input: ALL` — a passthrough participant's container answers `hitTest:` with
  `nil`, so the click reaches the framework's surface underneath.
- `focusable: true` — the participant, or failing that its container, becomes the
  window's first responder, which is what makes the OS hand it the keyboard and
  the IME.
- `snapshot: true` — a participant can be frozen into a texture for a transition
  (task 16.4 step 3). The bitmap round trip lives in
  `NativeViewHost::snapshot_view`: it reads the view's rendered pixels back out of
  the window server and stashes them for the app runner to import as an external
  texture.
