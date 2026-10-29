package dev.floom.plugin.background_work;

import android.app.Activity;
import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;

public final class BackgroundWorkPlugin {
    private BackgroundWorkPlugin() {}

    // `AlarmManager.RTC_WAKEUP` — wake the device to fire.
    private static final int RTC_WAKEUP = 0;

    // `PendingIntent.FLAG_UPDATE_CURRENT | FLAG_IMMUTABLE`.
    private static final int PENDING_FLAGS = 0x0800_0000 | 0x0400_0000;

    public static boolean scheduleAppLaunch(Context context, long atMillis) {
        Activity activity;
        if (!(context instanceof Activity)) {
            return false;
        }
        activity = (Activity) context;

        Intent intent = new Intent(activity, activity.getClass());
        PendingIntent pending;
        try {
            pending = PendingIntent.getActivity(
                    activity, 0, intent, PENDING_FLAGS);
        } catch (Throwable unreachable) {
            return false;
        }
        if (pending == null) {
            return false;
        }

        AlarmManager alarm = (AlarmManager) activity.getSystemService(Context.ALARM_SERVICE);
        if (alarm == null) {
            return false;
        }
        try {
            alarm.setAndAllowWhileIdle(RTC_WAKEUP, atMillis, pending);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }

    public static boolean cancelScheduledAppLaunch(Context context) {
        Activity activity;
        if (!(context instanceof Activity)) {
            return false;
        }
        activity = (Activity) context;

        Intent intent = new Intent(activity, activity.getClass());
        PendingIntent pending;
        try {
            pending = PendingIntent.getActivity(
                    activity, 0, intent, PENDING_FLAGS);
        } catch (Throwable unreachable) {
            return false;
        }
        if (pending == null) {
            return false;
        }

        AlarmManager alarm = (AlarmManager) activity.getSystemService(Context.ALARM_SERVICE);
        if (alarm == null) {
            return false;
        }
        try {
            alarm.cancel(pending);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }
}
