// libfloom_entry.so exposes no ArkTS-callable functions: it binds the
// XComponent surface in its N-API Init (via `libraryname`), then drives frames
// natively. The declaration file exists only so the module is recognised.
