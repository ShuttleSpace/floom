package dev.floom.plugin.wakelock;

import android.app.Activity;
import android.content.Context;
import android.view.WindowManager;

public final class WakelockPlugin {
    private WakelockPlugin() {}

    public static boolean keepScreenOn(Context context, boolean on) {
        if (!(context instanceof Activity)) {
            return false;
        }
        android.view.Window window = ((Activity) context).getWindow();
        if (window == null) {
            return false;
        }
        int flag = WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON;
        try {
            if (on) {
                window.addFlags(flag);
            } else {
                window.clearFlags(flag);
            }
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }

    public static boolean isKeepScreenOn(Context context) {
        if (!(context instanceof Activity)) {
            return false;
        }
        android.view.Window window = ((Activity) context).getWindow();
        if (window == null) {
            return false;
        }
        try {
            int flags = window.getAttributes().flags;
            return (flags & WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) != 0;
        } catch (Throwable unreachable) {
            return false;
        }
    }
}
