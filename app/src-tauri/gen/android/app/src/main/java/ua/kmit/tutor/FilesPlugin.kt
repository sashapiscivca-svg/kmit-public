package ua.kmit.tutor

import android.app.Activity
import android.content.Intent
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.speech.tts.TextToSpeech
import android.view.WindowManager
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import android.util.Base64
import java.util.Locale
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class SecureArgs {
    var on: Boolean = true
}

@InvokeArg
class KeyArgs {
    var slot: Int = 0
    var data: String = ""
}

@InvokeArg
class SpeakArgs {
    lateinit var text: String
}

@InvokeArg
class SaveFileArgs {
    lateinit var name: String
    lateinit var mime: String
    lateinit var data: String
}

/**
 * Зберігає файл (аркуш для вчителя, копію профілю) через системне вікно «Зберегти як».
 * Не потребує жодних дозволів і не бачить решти файлів пристрою. Не перевірено на пристрої (Q28).
 */
@TauriPlugin
class FilesPlugin(private val activity: Activity) : Plugin(activity) {
    private var pending: ByteArray? = null
    private var tts: TextToSpeech? = null
    private var ttsReady = false

    /** Озвучення вбудованим синтезатором телефона (рішення A18). Українська потребує встановленого голосу; не перевірено. */
    @Command
    fun speak(invoke: Invoke) {
        val args = invoke.parseArgs(SpeakArgs::class.java)
        val say = {
            val engine = tts
            if (engine != null) {
                engine.language = Locale("uk", "UA")
                engine.speak(args.text.take(600), TextToSpeech.QUEUE_FLUSH, null, "kmit")
            }
        }
        if (ttsReady) {
            say()
            invoke.resolve(JSObject().put("ok", true))
        } else {
            tts = TextToSpeech(activity) { status ->
                ttsReady = status == TextToSpeech.SUCCESS
                if (ttsReady) say()
            }
            invoke.resolve(JSObject().put("ok", true))
        }
    }

    /** Дозволити чи заборонити знімки екрана (FLAG_SECURE). За замовчуванням заборонено в MainActivity. */
    @Command
    fun setSecure(invoke: Invoke) {
        val args = invoke.parseArgs(SecureArgs::class.java)
        activity.runOnUiThread {
            if (args.on) {
                activity.window.setFlags(WindowManager.LayoutParams.FLAG_SECURE, WindowManager.LayoutParams.FLAG_SECURE)
            } else {
                activity.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
            }
        }
        invoke.resolve(JSObject().put("ok", true))
    }

    private fun slotKey(slot: Int, create: Boolean): SecretKey? {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val alias = "kmit_slot_$slot"
        (ks.getKey(alias, null) as? SecretKey)?.let { return it }
        if (!create) return null
        val spec = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply { init(spec) }.generateKey()
    }

    /** Обгортає ключ профілю апаратним ключем пристрою (AES-GCM у Android Keystore): iv(12) + шифртекст. Не перевірено на пристрої. */
    @Command
    fun wrapKey(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeyArgs::class.java)
            val key = slotKey(args.slot, true) ?: return invoke.reject("немає ключа")
            val c = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key) }
            val out = c.iv + c.doFinal(Base64.decode(args.data, Base64.NO_WRAP))
            invoke.resolve(JSObject().put("data", Base64.encodeToString(out, Base64.NO_WRAP)))
        } catch (e: Exception) {
            invoke.reject("wrap: ${e.javaClass.simpleName}")
        }
    }

    @Command
    fun unwrapKey(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeyArgs::class.java)
            val key = slotKey(args.slot, false) ?: return invoke.reject("немає ключа")
            val raw = Base64.decode(args.data, Base64.NO_WRAP)
            if (raw.size < 13) return invoke.reject("коротке значення")
            val c = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, raw.copyOfRange(0, 12))) }
            invoke.resolve(JSObject().put("data", Base64.encodeToString(c.doFinal(raw.copyOfRange(12, raw.size)), Base64.NO_WRAP)))
        } catch (e: Exception) {
            invoke.reject("unwrap: ${e.javaClass.simpleName}")
        }
    }

    @Command
    fun destroyKey(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(KeyArgs::class.java)
            KeyStore.getInstance("AndroidKeyStore").apply { load(null) }.deleteEntry("kmit_slot_${args.slot}")
            invoke.resolve(JSObject().put("ok", true))
        } catch (e: Exception) {
            invoke.reject("destroy: ${e.javaClass.simpleName}")
        }
    }

    @Command
    fun stopSpeaking(invoke: Invoke) {
        tts?.stop()
        invoke.resolve(JSObject().put("ok", true))
    }

    override fun onDestroy() {
        tts?.shutdown()
        tts = null
        ttsReady = false
        super.onDestroy()
    }

    @Command
    fun saveFile(invoke: Invoke) {
        val args = invoke.parseArgs(SaveFileArgs::class.java)
        pending = Base64.decode(args.data, Base64.DEFAULT)
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = args.mime
            putExtra(Intent.EXTRA_TITLE, args.name)
        }
        startActivityForResult(invoke, intent, "saveResult")
    }

    @ActivityCallback
    fun saveResult(invoke: Invoke, result: ActivityResult) {
        val bytes = pending
        pending = null
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null || bytes == null) {
            invoke.resolve(JSObject().put("saved", false))
            return
        }
        try {
            activity.contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) }
            invoke.resolve(JSObject().put("saved", true))
        } catch (e: Exception) {
            invoke.reject("Не вдалося записати файл", e)
        }
    }
}
