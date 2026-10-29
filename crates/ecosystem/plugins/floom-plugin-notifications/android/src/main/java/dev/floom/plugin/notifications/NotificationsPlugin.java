package dev.floom.plugin.notifications;

import android.Manifest;
import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Context;
import android.content.pm.PackageManager;
import android.os.Build;

public final class NotificationsPlugin {
    private NotificationsPlugin() {}

    /// `Build.VERSION_CODES.TIRAMISU`, the API level that introduced the runtime
    /// `POST_NOTIFICATIONS` permission.
    private static final int TIRAMISU = 33;

    /// `NotificationManager.areNotificationsEnabled()` — the system's own answer.
    public static boolean areEnabled(Context context) {
        NotificationManager manager =
                (NotificationManager) context.getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager == null) {
            return false;
        }
        try {
            return manager.areNotificationsEnabled();
        } catch (Throwable unreachable) {
            return false;
        }
    }

    /// Issues the real permission request on API 33+. Returns `true` when a
    /// request was issued or the permission is already granted; `false` on error.
    public static boolean requestAuthorization(Context context) {
        if (Build.VERSION.SDK_INT < TIRAMISU) {
            // Before API 33 there is no runtime permission and therefore no
            // prompt: an app may post as soon as it is installed, unless the
            // user has switched its notifications off in Settings.
            return areEnabled(context);
        }
        if (!(context instanceof Activity)) {
            return false;
        }
        Activity activity = (Activity) context;
        if (activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                == PackageManager.PERMISSION_GRANTED) {
            return true;
        }
        try {
            activity.requestPermissions(
                    new String[] {Manifest.permission.POST_NOTIFICATIONS}, 0);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }

    /// Builds a channel (API 26+) and posts the notification immediately.
    public static boolean schedule(
            Context context, String title, String body, int badge, boolean sound) {
        NotificationManager manager =
                (NotificationManager) context.getSystemService(Context.NOTIFICATION_SERVICE);
        if (manager == null) {
            return false;
        }
        try {
            Notification.Builder builder = new Notification.Builder(context, "floom");
            builder.setContentTitle(title);
            if (body != null && !body.isEmpty()) {
                builder.setContentText(body);
            }
            if (badge > 0) {
                builder.setNumber(badge);
            }
            builder.setChannelId("floom");
            if (sound) {
                builder.setSound(android.provider.Settings.System.DEFAULT_NOTIFICATION_URI);
            }
            ensureChannel(manager, sound);
            Notification built = builder.build();
            manager.notify("floom-notification", 1, built);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }

    private static void ensureChannel(NotificationManager manager, boolean sound) {
        if (Build.VERSION.SDK_INT < 26) {
            return;
        }
        NotificationChannel channel =
                new NotificationChannel("floom", "Floom", NotificationManager.IMPORTANCE_DEFAULT);
        if (sound) {
            channel.enableVibration(true);
        }
        manager.createNotificationChannel(channel);
    }
}
