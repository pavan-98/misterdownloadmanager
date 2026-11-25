package com.misterdownloadmanager.android

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.View
import androidx.activity.OnBackPressedCallback
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity

/**
 * The app's one activity: GameActivity loads libmdm.so (the manifest's `android.app.lib_name`)
 * and runs its `android_main` on a thread of its own, where MDM's interface draws (crates/android).
 */
class MainActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // The keystore, alarms, locks and the downloads' service, for the app (libmdm.so).
        Platform.init(this)
        nativeHost(Platform)
        // Android 13 and later show the downloads' notification only with this permission;
        // Android 10 and older write to the Download folder only with storage's.
        val wanted = when {
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU -> Manifest.permission.POST_NOTIFICATIONS
            Build.VERSION.SDK_INT <= Build.VERSION_CODES.Q -> Manifest.permission.WRITE_EXTERNAL_STORAGE
            else -> null
        }
        if (wanted != null && checkSelfPermission(wanted) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(wanted), 1)
        }
        // The interface draws inside the status and navigation bars, the display's cutout and the
        // keyboard, not under them: the content is padded by them, and the surface MDM draws on
        // with it.
        val content = findViewById<View>(android.R.id.content)
        ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
            )
            val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
            view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, ime.bottom))
            WindowInsetsCompat.CONSUMED
        }
        // Back (the gesture, the navigation bar's button) closes the window in front, as Esc
        // does on the desktop.
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() = nativeBack()
        })
    }

    /** The soft keyboard's action key (Done, Go): Enter for the app's text box. */
    override fun onEditorAction(action: Int) {
        nativeEditorAction()
        super.onEditorAction(action)
    }

    override fun onDestroy() {
        // GameActivity's destruction waits for the app's thread to end: the app is told first,
        // and closes (saving the list, as Exit does).
        nativeEnding()
        super.onDestroy()
    }

    private external fun nativeHost(platform: Any)

    private external fun nativeBack()

    private external fun nativeEnding()

    private external fun nativeEditorAction()
}
