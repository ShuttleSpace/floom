package dev.floom.plugin.local_auth;

import android.app.Activity;
import android.app.KeyguardManager;
import android.content.Context;
import android.content.pm.PackageManager;

public final class LocalAuthPlugin {
    public static final int BIOMETRIC_NONE = 0;
    public static final int BIOMETRIC_FINGERPRINT = 1;

    private LocalAuthPlugin() {}

    /// `KeyguardManager.isDeviceSecure()` (with `isKeyguardSecure()` fallback)
    /// plus a `FEATURE_FINGERPRINT` check. Returns one of `BIOMETRIC_*`.
    public static int availableBiometric(Context context) {
        KeyguardManager kg =
                (KeyguardManager) context.getSystemService(Context.KEYGUARD_SERVICE);
        if (kg == null) {
            return BIOMETRIC_NONE;
        }
        boolean secure;
        try {
            secure = kg.isDeviceSecure();
        } catch (Throwable unreachable) {
            secure = false;
        }
        if (!secure) {
            return BIOMETRIC_NONE;
        }
        if (context.getPackageManager()
                .hasSystemFeature(PackageManager.FEATURE_FINGERPRINT)) {
            return BIOMETRIC_FINGERPRINT;
        }
        return BIOMETRIC_NONE;
    }

    /// `createConfirmDeviceCredentialIntent` + `startActivityForResult`. Returns
    /// `true` when the prompt was launched.
    public static boolean authenticate(Context context, String reason) {
        KeyguardManager kg =
                (KeyguardManager) context.getSystemService(Context.KEYGUARD_SERVICE);
        if (kg == null) {
            return false;
        }
        if (!(context instanceof Activity)) {
            return false;
        }
        Activity activity = (Activity) context;
        android.content.Intent intent =
                kg.createConfirmDeviceCredentialIntent(null, null);
        if (intent == null) {
            return false;
        }
        try {
            activity.startActivityForResult(intent, 0);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }
}
