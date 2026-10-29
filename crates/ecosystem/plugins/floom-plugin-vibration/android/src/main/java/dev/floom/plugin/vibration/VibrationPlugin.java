package dev.floom.plugin.vibration;

import android.content.Context;
import android.os.Build;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.view.View;

public final class VibrationPlugin {
    private VibrationPlugin() {}

    public static boolean vibrate(Context context, long durationMillis) {
        Vibrator vibrator = (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);
        if (vibrator == null) {
            return false;
        }
        try {
            if (Build.VERSION.SDK_INT >= 26) {
                vibrator.vibrate(VibrationEffect.createOneShot(durationMillis, VibrationEffect.DEFAULT_AMPLITUDE));
            } else {
                vibrator.vibrate(durationMillis);
            }
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }

    public static boolean hapticFeedback(Context context, int constant) {
        if (!(context instanceof android.app.Activity)) {
            return false;
        }
        android.app.Activity activity = (android.app.Activity) context;
        View decor = activity.getWindow().getDecorView();
        if (decor == null) {
            return false;
        }
        try {
            return decor.performHapticFeedback(constant);
        } catch (Throwable unreachable) {
            return false;
        }
    }

    public static boolean hasVibrator(Context context) {
        Vibrator vibrator = (Vibrator) context.getSystemService(Context.VIBRATOR_SERVICE);
        if (vibrator == null) {
            return false;
        }
        try {
            return vibrator.hasVibrator();
        } catch (Throwable unreachable) {
            return false;
        }
    }
}
