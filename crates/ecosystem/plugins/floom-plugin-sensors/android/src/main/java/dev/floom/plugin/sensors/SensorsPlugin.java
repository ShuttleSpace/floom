package dev.floom.plugin.sensors;

import android.content.Context;
import android.hardware.Sensor;
import android.hardware.SensorManager;

public final class SensorsPlugin {
    private SensorsPlugin() {}

    /// `SensorManager.getDefaultSensor(type) != null`.
    public static boolean hasSensor(Context context, int type) {
        SensorManager manager =
                (SensorManager) context.getSystemService(Context.SENSOR_SERVICE);
        if (manager == null) {
            return false;
        }
        try {
            Sensor sensor = manager.getDefaultSensor(type);
            return sensor != null;
        } catch (Throwable unreachable) {
            return false;
        }
    }
}
