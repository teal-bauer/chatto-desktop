package run.chatto.desktop

import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {

  /** Exposed to JS as window.ChattoAndroid */
  private inner class ChattoJsBridge {
    @JavascriptInterface
    fun setActiveRoom(roomId: String) {
      NotificationService.activeRoomId = roomId.ifBlank { null }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    // Opt out of edge-to-edge — let the system handle keyboard resize
    WindowCompat.setDecorFitsSystemWindows(window, true)
    updateStatusBarTheme(resources.configuration)

    // Request notification permission on Android 13+
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 0)
    }

    // Attach JS bridge to the WebView once it's available
    window.decorView.post { attachJsBridge() }

    handleNavigateIntent(intent)
  }

  private fun attachJsBridge() {
    val webView = findWebView(findViewById(android.R.id.content))
    webView?.addJavascriptInterface(ChattoJsBridge(), "ChattoAndroid")
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    handleNavigateIntent(intent)
  }

  override fun onResume() {
    super.onResume()
    // Start the background notification service — it will connect once
    // the user is logged in and cookies are available
    NotificationService.start(this)
  }

  override fun onPause() {
    super.onPause()
    // When app is backgrounded, clear active room so all notifications fire
    NotificationService.activeRoomId = null
  }

  private fun handleNavigateIntent(intent: Intent?) {
    val url = intent?.getStringExtra("navigate_url") ?: return
    intent.removeExtra("navigate_url") // consume it
    // Delay slightly to ensure the WebView is ready
    window.decorView.postDelayed({
      findWebView(findViewById(android.R.id.content))
        ?.evaluateJavascript("window.location.href='$url'", null)
    }, 500)
  }

  private fun findWebView(view: android.view.View?): android.webkit.WebView? {
    if (view is android.webkit.WebView) return view
    if (view is android.view.ViewGroup) {
      for (i in 0 until view.childCount) {
        val found = findWebView(view.getChildAt(i))
        if (found != null) return found
      }
    }
    return null
  }

  override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    updateStatusBarTheme(newConfig)
  }

  private fun updateStatusBarTheme(config: Configuration) {
    val isDarkMode = (config.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
      Configuration.UI_MODE_NIGHT_YES
    val controller = WindowInsetsControllerCompat(window, window.decorView)
    // Light status bar = dark icons (for light backgrounds)
    controller.isAppearanceLightStatusBars = !isDarkMode
    window.statusBarColor = if (isDarkMode) Color.parseColor("#171717") else Color.WHITE
  }
}
