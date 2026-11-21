package dev.flowlist.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.flowlist.android.data.Repository
import dev.flowlist.android.ui.MainScreen
import dev.flowlist.android.ui.PairingScreen
import dev.flowlist.android.ui.FlowlistTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            FlowlistApp()
        }
    }
}

@Composable
private fun FlowlistApp() {
    val context = LocalContext.current
    val repository = remember { Repository(context.applicationContext) }
    val state by repository.state.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    FlowlistTheme {
        if (!state.paired) {
            PairingScreen(
                busy = state.syncing,
                error = state.error,
                onPair = { address, code ->
                    scope.launch { repository.pair(address, code) }
                },
            )
        } else {
            MainScreen(repository = repository, state = state, scope = scope)
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

    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        if (state.paired) {
            scope.launch { repository.sync() }
        }
    }
}
