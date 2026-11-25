package com.misterdownloadmanager.android

import com.google.androidgamesdk.GameActivity

/**
 * The app's one activity: GameActivity loads libmdm.so (the manifest's `android.app.lib_name`)
 * and runs its `android_main` on a thread of its own, where MDM's interface draws (crates/android).
 */
class MainActivity : GameActivity()
