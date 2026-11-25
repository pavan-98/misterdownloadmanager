package com.misterdownloadmanager.android

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

    private external fun nativeBack()
}
