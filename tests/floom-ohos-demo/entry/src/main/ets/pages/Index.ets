@Entry
@Component
struct Index {
  build() {
    Column() {
      // `libraryname` is the seam: the ArkUI runtime loads libfloom_entry.so and
      // injects OH_NATIVE_XCOMPONENT_OBJ into its N-API exports at Init time,
      // where the bridge registers the surface callbacks (see Flutter's
      // FlutterPage.ets / XComponentAdapter::Export). onLoad is not used to
      // pass the handle.
      XComponent({ id: 'floom', type: XComponentType.SURFACE, libraryname: 'floom_entry' })
        .width('100%')
        .height('100%')
    }
    .width('100%')
    .height('100%')
  }
}
