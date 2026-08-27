package ai.cyb.app

import android.os.Bundle
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity

/**
 * The one Activity. GameActivity loads `libcyb.so` (named by the
 * `android.app.lib_name` meta-data in the manifest) and hands the app to
 * Rust's `android_main` — the same Bevy app the desktop runs.
 *
 * GameActivity always renders edge to edge and does not honour
 * `setDecorFitsSystemWindows`, so the window really does extend under the
 * status bar and the gesture pill. The system reports how far in each
 * direction; this forwards those numbers to Rust, which is the only side
 * that knows what it drew there.
 */
class MainActivity : GameActivity() {
    companion object {
        init {
            System.loadLibrary("cyb")
        }
    }

    private external fun nativeSetInsets(top: Int, bottom: Int, ime: Int)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { _, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
            )
            val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
            nativeSetInsets(bars.top, bars.bottom, ime.bottom)
            insets
        }
    }
}
