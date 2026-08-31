package dev.floem.android;

import android.content.Context;
import android.os.Bundle;
import android.view.View;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityNodeProvider;
import java.util.List;

/**
 * Transparent, non-interactive overlay View that exposes Floem's accessibility
 * tree to Android's AccessibilityService (TalkBack) via a virtual-node
 * {@link AccessibilityNodeProvider}.
 *
 * <p>Floem injects one of these into the Activity's decor view from Rust (see
 * {@code src/accessibility/backend_android.rs}). It deliberately does NOT
 * consume touch events ({@code dispatchTouchEvent} returns false by default for
 * a View with no touch handling) so it never interferes with the rendering
 * surface below it, yet it still answers accessibility queries — the standard
 * pattern behind AndroidX's {@code ExploreByTouchHelper}.</p>
 *
 * <p>All heavy lifting happens in native code (the {@code native*} methods),
 * which read Floem's neutral a11y tree and build {@link AccessibilityNodeInfo}
 * objects / route actions back to Floem core through the shared
 * {@code PendingAction} channel.</p>
 *
 * <h3>App integration</h3>
 * Copy this file to your app's
 * {@code app/src/main/java/dev/floem/android/FloemAccessibilityView.java}
 * (the package name is load-bearing — it must stay {@code dev.floem.android} so
 * the JNI symbols resolve) and ensure the Rust library is loaded
 * ({@code System.loadLibrary}) before the window is created. Floem auto-injects
 * the view; no further Java wiring is required.
 */
public final class FloemAccessibilityView extends View {
    private AccessibilityNodeProvider provider;

    public FloemAccessibilityView(Context context) {
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
            provider = new FloemProvider();
        }
        return provider;
    }

    /** Ask TalkBack to re-read the tree. Safe to call from any thread. */
    public void requestA11yRefresh() {
        post(() -> sendAccessibilityEvent(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED));
    }

    private final class FloemProvider extends AccessibilityNodeProvider {
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
