package dev.floem.android;

import android.app.NativeActivity;
import android.content.Intent;

/**
 * Optional {@link NativeActivity} subclass that forwards {@code onActivityResult}
 * (Storage Access Framework file-picker results) and {@code onNewIntent} (files
 * shared into a running app) to Floem.
 *
 * <p>The default Floem example manifest uses {@code android.app.NativeActivity}
 * and works for everything except receiving file-dialog results and runtime
 * file shares. To enable either feature:
 *
 * <ol>
 *   <li>Add this file to your Android app's Java source set, e.g.
 *       {@code app/src/main/java/dev/floem/android/FloemNativeActivity.java}.</li>
 *   <li>In your {@code AndroidManifest.xml}, replace the activity
 *       {@code android:name="android.app.NativeActivity"} with
 *       {@code android:name="dev.floem.android.FloemNativeActivity"} and remove
 *       {@code android:hasCode="false"} (there is now Java code).</li>
 *   <li>Rebuild the Rust shared library with {@code cargo ndk} and repackage the
 *       APK (the {@code floemOnActivityResult} / {@code floemOnNewIntent} native
 *       methods are exported from that {@code .so}).</li>
 * </ol>
 *
 * <p>The native methods are resolved by the JVM via JNI's default dynamic linking
 * because they are exported (name-mangled) from the Rust library.
 */
public class FloemNativeActivity extends NativeActivity {
    public native void floemOnActivityResult(int requestCode, int resultCode, Intent data);
    public native void floemOnNewIntent(Intent intent);

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        floemOnActivityResult(requestCode, resultCode, data);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        floemOnNewIntent(intent);
    }
}
