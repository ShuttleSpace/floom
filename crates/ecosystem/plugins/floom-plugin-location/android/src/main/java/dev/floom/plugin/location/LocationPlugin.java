package dev.floom.plugin.location;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageManager;
import android.location.Location;
import android.location.LocationManager;

public final class LocationPlugin {
    // Mirrors `floom_capabilities::LocationAuthorization` discriminants.
    public static final int AUTH_NOT_DETERMINED = 0;
    public static final int AUTH_WHEN_IN_USE = 1;
    public static final int AUTH_ALWAYS = 2;
    public static final int AUTH_DENIED = 3;
    public static final int AUTH_RESTRICTED = 4;

    private LocationPlugin() {}

    /// A real permission check via `Context.checkSelfPermission`, mapped onto the
    /// crate's enum. Android has no "when in use" vs "always" split in the
    /// permission string for *foreground* access, so a granted
    /// `ACCESS_FINE_LOCATION` is reported as `AUTH_WHEN_IN_USE` and a granted
    /// `ACCESS_BACKGROUND_LOCATION` as `AUTH_ALWAYS`.
    public static int authorizationStatus(Context context) {
        int fine = context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION);
        if (fine != PackageManager.PERMISSION_GRANTED) {
            // Denied or never asked: Android does not distinguish "denied" from
            // "not yet asked" through this API, and `shouldShowRequestPermissionRationale`
            // is the only signal — it returns false for both. `AUTH_NOT_DETERMINED`
            // keeps the caller's next step (ask) correct either way.
            return AUTH_NOT_DETERMINED;
        }
        int background =
                context.checkSelfPermission(Manifest.permission.ACCESS_BACKGROUND_LOCATION);
        if (background == PackageManager.PERMISSION_GRANTED) {
            return AUTH_ALWAYS;
        }
        return AUTH_WHEN_IN_USE;
    }

    /// `LocationManager.isLocationEnabled()` (API 28+), falling back to
    /// `isProviderEnabled(GPS_PROVIDER)` on older devices. This is the
    /// system-wide location switch, not the app's permission.
    public static boolean isLocationEnabled(Context context) {
        LocationManager manager =
                (LocationManager) context.getSystemService(Context.LOCATION_SERVICE);
        if (manager == null) {
            return false;
        }
        try {
            return manager.isLocationEnabled();
        } catch (Throwable unreachable) {
            try {
                return manager.isProviderEnabled(LocationManager.GPS_PROVIDER);
            } catch (Throwable e2) {
                return false;
            }
        }
    }

    /// Issues the real `requestPermissions` call. Returns `true` when requested.
    public static boolean requestPermissions(Context context) {
        if (!(context instanceof Activity)) {
            return false;
        }
        Activity activity = (Activity) context;
        try {
            activity.requestPermissions(
                    new String[] {Manifest.permission.ACCESS_FINE_LOCATION}, 0);
            return true;
        } catch (Throwable unreachable) {
            return false;
        }
    }

    /// `getLastKnownLocation` is a synchronous read of the last fix the OS
    /// cached. It is *not* a fresh fix. `[lat, lng, alt, accuracy, timeMs]`, or
    /// `null` when nothing is cached.
    public static double[] currentLocation(Context context) {
        LocationManager manager =
                (LocationManager) context.getSystemService(Context.LOCATION_SERVICE);
        if (manager == null) {
            return null;
        }
        Location fix = manager.getLastKnownLocation(LocationManager.GPS_PROVIDER);
        if (fix == null) {
            fix = manager.getLastKnownLocation(LocationManager.NETWORK_PROVIDER);
        }
        if (fix == null) {
            return null;
        }
        return new double[] {
            fix.getLatitude(),
            fix.getLongitude(),
            fix.getAltitude(),
            fix.getAccuracy(),
            fix.getTime(),
        };
    }
}
