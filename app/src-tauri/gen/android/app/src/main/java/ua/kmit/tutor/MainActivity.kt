package ua.kmit.tutor

import android.os.Bundle
import android.view.WindowManager
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    // Приватність: знімки екрана й ескізи в списку останніх програм вимкнено за замовчуванням
    // (учень може дозволити знімки в налаштуваннях: команда setSecure).
    window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
    super.onCreate(savedInstanceState)
  }
}
