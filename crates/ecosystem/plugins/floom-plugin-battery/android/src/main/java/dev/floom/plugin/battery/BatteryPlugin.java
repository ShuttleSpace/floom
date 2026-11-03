package dev.floom.plugin.battery;

import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.os.BatteryManager;
import android.provider.Settings;

public final class BatteryPlugin {
    private BatteryPlugin() {}

    public static double[] snapshot(Context context) {
        try {
            Intent intent = context
                    .getApplicationContext()
                    .registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
            if (intent == null) {
                return new double[0];
            }
            int level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
            if (level < 0 || scale <= 0) {
                return new double[0];
            }
            int plugged = intent.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0);
            int status = intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            double fraction = Math.min(1.0, Math.max(0.0, (double) level / (double) scale));
            return new double[] {
                fraction,
                plugged != 0 ? 1.0 : 0.0,
                status == BatteryManager.BATTERY_STATUS_FULL ? 1.0 : 0.0,
            };
        } catch (Throwable unreadable) {
            return new double[0];
        }
    }

    public static boolean isLowPowerMode(Context context) {
        try {
            return Settings.Global.getInt(
                    context.getContentResolver(), "battery_saver_mode", 0) == 1;
        } catch (Throwable unreadable) {
            return false;
        }
    }
}
