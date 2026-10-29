package dev.floom.plugin.prefs;

import android.content.Context;
import android.content.SharedPreferences;

public final class PrefsPlugin {
    private PrefsPlugin() {}

    private static final int MODE_PRIVATE = 0;

    public static String get(Context context, String store, String key) {
        SharedPreferences sp = context.getSharedPreferences(store, MODE_PRIVATE);
        return sp.getString(key, null);
    }

    public static boolean set(Context context, String store, String key, String value) {
        SharedPreferences sp = context.getSharedPreferences(store, MODE_PRIVATE);
        SharedPreferences.Editor editor = sp.edit();
        editor.putString(key, value);
        return editor.commit();
    }

    public static boolean remove(Context context, String store, String key) {
        SharedPreferences sp = context.getSharedPreferences(store, MODE_PRIVATE);
        SharedPreferences.Editor editor = sp.edit();
        editor.remove(key);
        return editor.commit();
    }
}
