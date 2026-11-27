// libfloom_entry.so binds the XComponent surface in its N-API Init (via
// `libraryname`) and drives frames natively; the two functions below are the
// only ones ArkTS calls directly.
declare const api: {
  wantsIme(): boolean;
  back(): void;
};
export default api;
