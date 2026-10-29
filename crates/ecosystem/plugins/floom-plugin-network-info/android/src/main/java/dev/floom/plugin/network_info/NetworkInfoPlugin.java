package dev.floom.plugin.network_info;

import android.content.Context;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;

public final class NetworkInfoPlugin {
    private NetworkInfoPlugin() {}

    public static String[] wifiInfo(Context context) {
        WifiManager manager = (WifiManager) context.getSystemService(Context.WIFI_SERVICE);
        if (manager == null) {
            return new String[] { null, null };
        }
        WifiInfo info = manager.getConnectionInfo();
        if (info == null) {
            return new String[] { null, null };
        }
        String ssid = info.getSSID();
        if (ssid != null && ssid.equals("<unknown ssid>")) {
            ssid = null;
        } else if (ssid != null) {
            // A known SSID arrives wrapped in literal double quotes.
            ssid = ssid.replace("\"", "");
        }
        String bssid = info.getBSSID();
        return new String[] { ssid, bssid };
    }
}
