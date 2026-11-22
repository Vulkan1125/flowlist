package dev.flowlist.android

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.flowlist.android.data.AppPreferences
import dev.flowlist.android.data.PreferencesStore
import dev.flowlist.android.data.Repository
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.ui.FlowlistTheme
import dev.flowlist.android.ui.MainScreen
import dev.flowlist.android.data.SharedPayload
import dev.flowlist.android.notify.NearbyReminders
import dev.flowlist.android.ui.PairingScreen
import dev.flowlist.android.ui.isDark
import dev.flowlist.android.widget.FlowlistWidget
import dev.flowlist.android.notify.DueNotifications
import androidx.glance.appwidget.updateAll
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private var openTab by mutableStateOf<String?>(null)
    private var shared by mutableStateOf<SharedPayload?>(null)
    private var openMore by mutableStateOf<String?>(null)
    private var openCapture by mutableStateOf(false)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        openTab = intent.getStringExtra("tab")
        openMore = intent.getStringExtra("more")
        openCapture = intent?.getBooleanExtra("capture", false) == true
        shared = sharedPayload(intent)
        setContent {
            FlowlistApp(
                openTab = openTab,
                onTabConsumed = { openTab = null },
                shared = shared,
                onSharedConsumed = { shared = null },
                initialMore = openMore,
                onMoreConsumed = { openMore = null },
                initialCapture = openCapture,
                onCaptureConsumed = { openCapture = false },
            )
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        openTab = intent.getStringExtra("tab")
        openMore = intent.getStringExtra("more")
        openCapture = intent.getBooleanExtra("capture", false)
        shared = sharedPayload(intent)
    }

    /** What another app shared: a file, a link, or plain text. */
    private fun sharedPayload(intent: Intent?): SharedPayload? {
        if (intent == null) return null
        when (intent.action) {
            Intent.ACTION_SEND -> {
                (streamUri(intent) ?: intent.data)?.let { uri ->
                    if (uri.scheme != "flowlist") {
                        return SharedPayload.File(uri, displayName(uri))
                    }
                }
                val text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()?.trim()
                if (!text.isNullOrEmpty()) return classify(intent, text)
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                val uris = intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)
                val uri = uris?.firstOrNull()
                if (uri != null) return SharedPayload.File(uri, displayName(uri))
            }
            Intent.ACTION_PROCESS_TEXT -> {
                val text = intent.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)?.toString()?.trim()
                if (!text.isNullOrEmpty()) return SharedPayload.Text(text)
            }
        }
        return null
    }

    private fun classify(intent: Intent, text: String): SharedPayload {
        val url = Regex("""https?://\S+""").find(text)?.value
        val link = url != null && (text == url || (text.length <= 200 && text.endsWith(url)))
        if (!link || url == null) return SharedPayload.Text(text)
        val title = listOf(Intent.EXTRA_TITLE, Intent.EXTRA_SUBJECT)
            .firstNotNullOfOrNull { key ->
                intent.getCharSequenceExtra(key)?.toString()?.trim()?.takeIf { it.isNotEmpty() }
            }
            ?: text.replace(url, "").trim().takeIf { it.isNotEmpty() }
        return SharedPayload.Link(url, title)
    }

    private fun streamUri(intent: Intent): Uri? =
        if (Build.VERSION.SDK_INT >= 33) {
            intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        } else {
            @Suppress("DEPRECATION")
            intent.getParcelableExtra(Intent.EXTRA_STREAM)
        }

    private fun displayName(uri: Uri): String? = runCatching {
        contentResolver.query(
            uri,
            arrayOf(OpenableColumns.DISPLAY_NAME),
            null,
            null,
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        } ?: uri.lastPathSegment
    }.getOrNull()
}

@Composable
private fun FlowlistApp(
    openTab: String?,
    onTabConsumed: () -> Unit,
    shared: SharedPayload?,
    onSharedConsumed: () -> Unit,
    initialMore: String? = null,
    onMoreConsumed: () -> Unit = {},
    initialCapture: Boolean = false,
    onCaptureConsumed: () -> Unit = {},
) {
    val context = LocalContext.current
    val repository = remember { Repository(context.applicationContext) }
    val themeStore = remember { ThemeStore(context.applicationContext) }
    val preferencesStore = remember { PreferencesStore(context.applicationContext) }
    var theme by remember { mutableStateOf(themeStore.load()) }
    var preferences by remember { mutableStateOf(preferencesStore.load()) }
    val state by repository.state.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    val view = LocalView.current
    val dark = theme.isDark()

    SideEffect {
        val window = (view.context as? Activity)?.window ?: return@SideEffect
        WindowCompat.getInsetsController(window, view).apply {
            isAppearanceLightStatusBars = !dark
            isAppearanceLightNavigationBars = !dark
        }
    }

    FlowlistTheme(theme) {
        if (!state.paired) {
            PairingScreen(
                busy = state.syncing,
                error = state.error,
                onPair = { address, code ->
                    scope.launch { repository.pair(address, code) }
                },
                onPairBases = { addresses, code ->
                    scope.launch { repository.pairAny(addresses, code) }
                },
            )
        } else {
            MainScreen(
                repository = repository,
                state = state,
                scope = scope,
                theme = theme,
                onThemeChange = {
                    themeStore.save(it)
                    theme = it
                },
                initialTab = openTab,
                onTabConsumed = onTabConsumed,
                shared = shared,
                onSharedConsumed = onSharedConsumed,
                initialMore = initialMore,
                onMoreConsumed = onMoreConsumed,
                initialCapture = initialCapture,
                onCaptureConsumed = onCaptureConsumed,
                preferences = preferences,
                onPreferencesChange = {
                    preferencesStore.save(it)
                    preferences = it
                },
            )
        }
    }

    LaunchedEffect(Unit) {
        if (DueNotifications.enabled(context)) {
            DueNotifications.ensureChannel(context)
            DueNotifications.schedule(context)
        }
    }

    LaunchedEffect(state.paired) {
        if (!state.paired) return@LaunchedEffect
        repository.sync()
        while (true) {
            delay(20_000)
            repository.sync()
        }
    }

    LaunchedEffect(state.snapshot?.revision) {
        if (state.snapshot != null) {
            FlowlistWidget().updateAll(context.applicationContext)
            NearbyReminders.sync(context, state.snapshot!!.db)
        }
    }

    LaunchedEffect(theme) {
        FlowlistWidget().updateAll(context.applicationContext)
    }

    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        if (state.paired) {
            scope.launch { repository.sync() }
            state.snapshot?.let { NearbyReminders.sync(context, it.db) }
        }
    }
}
