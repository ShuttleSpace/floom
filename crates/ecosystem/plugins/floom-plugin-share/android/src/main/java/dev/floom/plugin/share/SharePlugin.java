package dev.floom.plugin.share;

import android.content.Context;
import android.content.Intent;

public final class SharePlugin {
    private SharePlugin() {}

    public static boolean shareText(Context context, String text) {
        if (text == null || text.isEmpty()) {
            return false;
        }
        try {
            Intent intent = new Intent(Intent.ACTION_SEND);
            intent.setType("text/plain");
            intent.putExtra(Intent.EXTRA_TEXT, text);
            Intent chooser = Intent.createChooser(intent, "Share via");
            if (chooser == null) {
                return false;
            }
            context.startActivity(chooser);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }
}
