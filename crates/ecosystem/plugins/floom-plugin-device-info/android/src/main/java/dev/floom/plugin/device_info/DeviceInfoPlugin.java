package dev.floom.plugin.device_info;

import android.os.Build;

import java.util.Locale;

public final class DeviceInfoPlugin {
    private DeviceInfoPlugin() {}

    /// `[model, manufacturer, fingerprint, osVersion, sdkInt, locale]`.
    ///
    /// Every field is `android.os.Build`; the OS version and API level live in
    /// the nested `Build.VERSION`. An empty string stands in for any field the
    /// device does not expose (an OEM that stripped it, a rename a toolchain
    /// applied) — the absence is a legitimate "I don't know", not an error.
    public static String[] basics() {
        String model = Build.MODEL != null ? Build.MODEL : "";
        String manufacturer = Build.MANUFACTURER != null ? Build.MANUFACTURER : "";
        String fingerprint = Build.FINGERPRINT != null ? Build.FINGERPRINT : "";
        String osVersion = Build.VERSION.RELEASE != null ? Build.VERSION.RELEASE : "";
        String sdkInt = String.valueOf(Build.VERSION.SDK_INT);
        Locale locale = Locale.getDefault();
        String localeTag = locale != null ? locale.toLanguageTag() : "";
        return new String[] {model, manufacturer, fingerprint, osVersion, sdkInt, localeTag};
    }
}
