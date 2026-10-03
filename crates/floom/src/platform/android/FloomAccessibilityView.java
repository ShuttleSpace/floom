package dev.floom.android;

import android.content.Context;
import android.os.Bundle;
import android.view.View;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityNodeProvider;
import java.util.List;

/**
 * Transparent, non-interactive overlay View that exposes Floom's accessibility
 * tree to Android's AccessibilityService (TalkBack) via a virtual-node
 * {@link AccessibilityNodeProvider}.
 *
 * <p>Floom injects one of these into the Activity's decor view from Rust (see
 * {@code crates/floom/src/accessibility/backend_android.rs}). It deliberately does NOT
 * consume touch events ({@code dispatchTouchEvent} returns false by default for
 * a View with no touch handling) so it never interferes with the rendering
 * surface below it, yet it still answers accessibility queries — the standard
 * pattern behind AndroidX's {@code ExploreByTouchHelper}.</p>
 *
 * <p>All heavy lifting happens in native code (the {@code native*} methods),
 * which read Floom's neutral a11y tree and build {@link AccessibilityNodeInfo}
 * objects / route actions back to Floom core through the shared
 * {@code PendingAction} channel.</p>
 *
 * <h3>App integration</h3>
 * Copy this file to your app's
 * {@code app/src/main/java/dev/floom/android/FloomAccessibilityView.java}
 * (the package name is load-bearing — it must stay {@code dev.floom.android} so
 * the JNI symbols resolve) and ensure the Rust library is loaded
 * ({@code System.loadLibrary}) before the window is created. Floom auto-injects
 * the view; no further Java wiring is required.
 */
public final class FloomAccessibilityView extends View {
    private AccessibilityNodeProvider provider;

    public FloomAccessibilityView(Context context) {
        super(context);
        setWillNotDraw(true);
        setClickable(false);
        setFocusable(false);
        setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        // Register this view with the native bridge (stores a global ref so the
        // native side can fire content-changed events later).
        nativeRegisterView();
    }

    @Override
    public AccessibilityNodeProvider getAccessibilityNodeProvider() {
        if (provider == null) {
            provider = new FloomProvider();
        }
        return provider;
    }

    /** Ask TalkBack to re-read the tree. Safe to call from any thread. */
    public void requestA11yRefresh() {
        post(() -> sendAccessibilityEvent(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED));
    }

    private final class FloomProvider extends AccessibilityNodeProvider {
        @Override
        public AccessibilityNodeInfo createAccessibilityNodeInfo(int virtualViewId) {
            return nativeCreateNodeInfo(virtualViewId);
        }

        @Override
        public List<AccessibilityNodeInfo> findAccessibilityNodeInfosByText(
                String text, int virtualViewId) {
            return nativeFindNodesByText(text, virtualViewId);
        }

        @Override
        public boolean performAction(int virtualViewId, int action, Bundle arguments) {
            return nativePerformAction(virtualViewId, action, arguments);
        }
    }

    private native void nativeRegisterView();

    private native AccessibilityNodeInfo nativeCreateNodeInfo(int virtualViewId);

    private native List<AccessibilityNodeInfo> nativeFindNodesByText(
            String text, int virtualViewId);

    private native boolean nativePerformAction(int virtualViewId, int action, Bundle arguments);
}
