package dev.floom.plugin.connectivity;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;

public final class ConnectivityPlugin {
    private ConnectivityPlugin() {}

    /// `[transportMask, hasInternet, isMetered]`.
    ///
    /// * `transportMask` — the `TRANSPORT_*` bits folded into one int:
    ///   `1 << TRANSPORT_CELLULAR` (0), `1 << TRANSPORT_WIFI` (1),
    ///   `1 << TRANSPORT_BLUETOOTH` (2), `1 << TRANSPORT_ETHERNET` (3).
    /// * `hasInternet` — `NET_CAPABILITY_INTERNET` present (0 or 1).
    /// * `isMetered` — `NET_CAPABILITY_NOT_METERED` *absent* (0 or 1); the
    ///   capability is phrased negatively, so its absence is metered.
    ///
    /// Returns `null` when there is no active network or capabilities could not
    /// be read.
    public static int[] capabilities(Context context) {
        ConnectivityManager manager =
                (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        if (manager == null) {
            return null;
        }
        Network network = manager.getActiveNetwork();
        if (network == null) {
            return null;
        }
        NetworkCapabilities caps = manager.getNetworkCapabilities(network);
        if (caps == null) {
            return null;
        }
        int mask = 0;
        // `hasTransport(int)` reports each `TRANSPORT_*` bit. Fold them into one
        // int so the Rust side can read one array and decide the transport.
        for (int t : new int[] {
            android.net.NetworkCapabilities.TRANSPORT_CELLULAR,
            android.net.NetworkCapabilities.TRANSPORT_WIFI,
            android.net.NetworkCapabilities.TRANSPORT_BLUETOOTH,
            android.net.NetworkCapabilities.TRANSPORT_ETHERNET,
        }) {
            if (caps.hasTransport(t)) {
                mask |= (1 << t);
            }
        }
        int hasInternet =
                caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) ? 1 : 0;
        int notMetered =
                caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) ? 1 : 0;
        int isMetered = notMetered != 0 ? 0 : 1;
        return new int[] {mask, hasInternet, isMetered};
    }
}
