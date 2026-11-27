import nativeApi from 'libfloom_entry.so';

@Entry
@Component
struct Index {
  private typed: string = '';

  // The emulator's input service refuses `uitest uiInput inputText` (error
  // 29201), so the keyboard half cannot be driven from a script yet. This pushes
  // one known composition instead, which proves the ETS -> napi -> Rust leg.
  aboutToAppear(): void {
    nativeApi.textInput('汉字 selfcheck');
    // Floom owns focus internally; the keyboard follows the caret the host
    // reports, polled here because there is no callback into ArkTS.
    setInterval(() => {
      if (nativeApi.wantsIme()) {
        focusControl.requestFocus('floom-ime');
      }
    }, 200);
  }

  build() {
    Stack() {
      // Transparent, unfocusable-by-touch, always focused: ArkUI is the only
      // side that can own a keyboard, so composition is read here and handed
      // to Rust (see Flutter's approach — a hidden TextInput over the surface).
      TextInput({ text: this.typed })
        .width(1)
        .height(1)
        .opacity(0)
        .hitTestBehavior(HitTestMode.None)
        .id('floom-ime')
        .onChange((value: string) => {
          nativeApi.textInput(value);
        })
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
    .width('100%')
    .height('100%')
  }
}
