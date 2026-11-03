package dev.floom.plugin.webview;

import android.app.Activity;
import android.content.Context;
import android.view.ViewGroup;
import android.webkit.WebView;

import android.view.FrameLayout;

/**
 * Floom native-view WebView: an {@link android.webkit.WebView} wrapped for Floom.
 *
 * The Rust half drives this through JNI: {@code create}, {@code loadUrl},
 * {@code evaluateJs}, {@code setPosition}, {@code setVisible}. The web view is
 * the native participant in the Floom scene; Floom controls its geometry through
 * the provider ABI and WebView renders the page.
 */
public class FloomWebView extends FrameLayout {
    private static final String TAG = "FloomWebView";

    private final Activity activity;
    private final WebView webView;

    public FloomWebView(Context context, Activity activity) {
        super(context);
        this.activity = activity;
        this.webView = new WebView(context);
        // A fresh document per view so the web view is never an empty view.
        webView.loadUrl("about:blank");
        addView(webView, new FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT));
    }

    /** Load a URL into the web view. */
    public void loadUrl(String url) {
        webView.loadUrl(url);
    }

    /** Evaluate JS in the page, for the Rust->JS bridge. */
    public void evaluateJs(String js) {
        webView.evaluateJavascript(js, value -> {
            // The result channel is not used by the bridge; the callback is
            // Fire-and-forget.
        });
    }

    /** Set position as x,y,width,height in px. */
    public void setPosition(float x, float y, float width, float height) {
        layout((int) x, (int) y, (int) width, (int) height);
    }

    /** Show or hide the web view. */
    public void setVisible(boolean visible) {
        setVisibility(visible ? VISIBLE : GONE);
    }

    private void layout(int x, int y, int width, int height) {
        ViewGroup.LayoutParams lp = getLayoutParams();
        if (lp == null) {
            lp = new FrameLayout.LayoutParams(width, height);
        }
        lp.left = x;
        lp.top = y;
        lp.width = width;
        lp.height = height;
        setLayoutParams(lp);
    }
}
