package ai.cyb.app

import com.google.androidgamesdk.GameActivity

/**
 * The one Activity. GameActivity loads `libcyb.so` (named by the
 * `android.app.lib_name` meta-data in the manifest) and hands the app to
 * Rust's `android_main` — the same Bevy app the desktop runs.
 */
class MainActivity : GameActivity() {
    companion object {
        init {
            System.loadLibrary("cyb")
        }
    }
}
