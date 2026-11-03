package dev.floom.plugin.camera;

import android.app.Activity;
import android.content.Context;
import android.hardware.camera2.*;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Log;
import android.util.Size;
import android.view.Surface;
import android.view.TextureView;
import android.view.ViewGroup;
import android.view.ViewGroup.LayoutParams;
import android.widget.FrameLayout;

import java.util.Arrays;

/**
 * Android half of the nativeview plugin's camera capability.
 *
 * A {@link TextureView}-backed view that opens a Camera2 preview session and
 * renders live frames into itself. The Rust half drives it through JNI:
 *
 *   {@link #openCamera(Activity)}  — open the first available camera and start preview
 *   {@link #closeCamera()}         — stop preview and release the camera
 *   {@link #setPosition(float,float,float,float)} — set X/Y/width/height in px
 *   {@link #setVisible(boolean)}   — show/hide the view
 *
 * Frames are rendered by Camera2 directly into the TextureView's SurfaceTexture,
 * so Rust never touches pixel data — the view is a native participant in the
 * Floom scene, and Floom controls its geometry through the provider ABI.
 */
public class FloomCameraView extends FrameLayout {
    private static final String TAG = "FloomCameraView";

    private final Activity activity;
    private final TextureView textureView;
    private CameraDevice cameraDevice;
    private CameraCaptureSession previewSession;
    private HandlerThread cameraThread;
    private Handler cameraHandler;
    private String cameraId;

    public FloomCameraView(Context context, Activity activity) {
        super(context);
        this.activity = activity;
        textureView = new TextureView(context);
        addView(textureView, new LayoutParams(
            LayoutParams.MATCH_PARENT,
            LayoutParams.MATCH_PARENT));
    }

    /**
     * Open the first available camera and start preview into this view.
     * Requires the CAMERA runtime permission to have been granted already.
     */
    public void openCamera() {
        cameraThread = new HandlerThread("Camera2");
        cameraThread.start();
        cameraHandler = new Handler(cameraThread.getLooper());

        CameraManager manager = (CameraManager) activity.getSystemService(Context.CAMERA_SERVICE);
        try {
            String[] ids = manager.getCameraIdList();
            if (ids.length == 0) {
                Log.e(TAG, "No cameras found");
                return;
            }
            // Prefer a rear-facing camera if one exists, otherwise the first.
            cameraId = ids[0];
            for (String id : ids) {
                try {
                    CameraCharacteristics c = manager.getCameraCharacteristics(id);
                    Integer facing = c.get(CameraCharacteristics.LENS_FACING);
                    if (facing != null && facing == CameraCharacteristics.LENS_FACING_BACK) {
                        cameraId = id;
                        break;
                    }
                } catch (CameraAccessException ignored) {
                }
            }

            CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
            StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            if (map == null) {
                Log.e(TAG, "No stream configuration map");
                return;
            }
            Size[] sizes = map.getOutputSizes(SurfaceTexture.class);
            Size previewSize = sizes.length > 0 ? sizes[0] : new Size(640, 480);

            textureView.setSurfaceTextureListener(new TextureView.SurfaceTextureListener() {
                @Override
                public void onSurfaceTextureAvailable(SurfaceTexture surface, int width, int height) {
                    Surface s = new Surface(surface);
                    try {
                        manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                            @Override
                            public void onOpened(CameraDevice camera) {
                                cameraDevice = camera;
                                try {
                                    cameraDevice.createCaptureSession(
                                        Arrays.asList(s),
                                        new CameraCaptureSession.StateCallback() {
                                            @Override
                                            public void onConfigured(CameraCaptureSession session) {
                                                previewSession = session;
                                                try {
                                                    CaptureRequest.Builder builder =
                                                        cameraDevice.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                                                    builder.addTarget(s);
                                                    session.setRepeatingRequest(builder.build(), null, cameraHandler);
                                                } catch (CameraAccessException e) {
                                                    Log.e(TAG, "Failed to set repeating request", e);
                                                }
                                            }
                                            @Override
                                            public void onConfigureFailed(CameraCaptureSession session) {
                                                Log.e(TAG, "Failed to configure capture session");
                                            }
                                        },
                                        cameraHandler
                                    );
                                } catch (CameraAccessException e) {
                                    Log.e(TAG, "Failed to create capture session", e);
                                }
                            }
                            @Override
                            public void onDisconnected(CameraDevice camera) {
                                camera.close();
                                cameraDevice = null;
                            }
                            @Override
                            public void onError(CameraDevice camera, int error) {
                                Log.e(TAG, "Camera error: " + error);
                                camera.close();
                                cameraDevice = null;
                            }
                        }, cameraHandler);
                    } catch (CameraAccessException e) {
                        Log.e(TAG, "Failed to open camera", e);
                    }
                }

                @Override
                public void onSurfaceTextureSizeChanged(SurfaceTexture surface, int width, int height) {
                }

                @Override
                public boolean onSurfaceTextureDestroyed(SurfaceTexture surface) {
                    return true;
                }

                @Override
                public void onSurfaceTextureUpdated(SurfaceTexture surface) {
                    // A new frame has been rendered into the TextureView.
                    // Rust is notified through the provider's apply/place path;
                    // no per-frame callback into Rust is needed for display.
                }
            });
        } catch (CameraAccessException e) {
            Log.e(TAG, "Failed to get camera manager", e);
        }
    }

    public void closeCamera() {
        if (previewSession != null) {
            try {
                previewSession.close();
            } catch (Exception ignored) {
            }
            previewSession = null;
        }
        if (cameraDevice != null) {
            cameraDevice.close();
            cameraDevice = null;
        }
        if (cameraThread != null) {
            cameraThread.quitSafely();
            try {
                cameraThread.join(2000);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
            cameraThread = null;
        }
        cameraHandler = null;
    }

    public void setPosition(float x, float y, float width, float height) {
        setX(x);
        setY(y);
        LayoutParams params = getLayoutParams();
        if (params == null) {
            params = new LayoutParams((int) width, (int) height);
        } else {
            params.width = (int) width;
            params.height = (int) height;
        }
        setLayoutParams(params);
    }

    public void setVisible(boolean visible) {
        setVisibility(visible ? VISIBLE : INVISIBLE);
    }

    public boolean isCameraOpen() {
        return cameraDevice != null;
    }
}
