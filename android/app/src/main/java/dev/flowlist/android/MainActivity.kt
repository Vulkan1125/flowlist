package dev.flowlist.android

import android.app.Activity
import android.content.Intent
import android.os.Bundle
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
import dev.flowlist.android.ui.PairingScreen
import dev.flowlist.android.ui.isDark
import dev.flowlist.android.widget.FlowlistWidget
import dev.flowlist.android.notify.DueNotifications
import androidx.glance.appwidget.updateAll
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private var openTab by mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        openTab = intent.getStringExtra("tab")
        setContent {
            FlowlistApp(openTab = openTab, onTabConsumed = { openTab = null })
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        openTab = intent.getStringExtra("tab")
    }
}

@Composable
private fun FlowlistApp(openTab: String?, onTabConsumed: () -> Unit) {
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
        }
    }

    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        if (state.paired) {
            scope.launch { repository.sync() }
        }
    }
}
