package dev.flowlist.android.notify

import android.app.Service
import android.content.Intent
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.IBinder
import android.provider.Settings
import android.view.Gravity
import android.view.WindowManager
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.core.app.NotificationManagerCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.lifecycle.setViewTreeViewModelStoreOwner
import androidx.savedstate.SavedStateRegistry
import androidx.savedstate.SavedStateRegistryController
import androidx.savedstate.SavedStateRegistryOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import dev.flowlist.android.FlowlistApplication
import dev.flowlist.android.MainActivity
import dev.flowlist.android.data.PreferencesStore
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.ui.AppBackground
import dev.flowlist.android.ui.AppearanceMode
import dev.flowlist.android.ui.FlowlistTheme
import dev.flowlist.android.ui.reminderBackgroundFor
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/**
 * Draws the full-screen reminder as a system overlay while the phone is
 * unlocked and another app owns the screen. Full-screen intents are only
 * allowed to raise an activity on the lock screen; this is the sanctioned way
 * to cover an app such as YouTube, and it needs "display over other apps".
 */
class ReminderOverlayService :
    Service(),
    LifecycleOwner,
    SavedStateRegistryOwner,
    ViewModelStoreOwner {
    private val lifecycleRegistry = LifecycleRegistry(this)
    private val savedStateController = SavedStateRegistryController.create(this)
    private val store = ViewModelStore()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var windowManager: WindowManager? = null
    private var overlay: ComposeView? = null
    private var audioFocusRequest: AudioFocusRequest? = null
    private var polling = false
    private var taskId = ""
    private var title = "Task due"
    private var project: String? = null
    private var dueAt = 0L

    override val lifecycle: Lifecycle get() = lifecycleRegistry
    override val savedStateRegistry: SavedStateRegistry
        get() = savedStateController.savedStateRegistry
    override val viewModelStore: ViewModelStore get() = store

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        savedStateController.performAttach()
        savedStateController.performRestore(null)
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_CREATE)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val id = intent?.getStringExtra("task_id") ?: run {
            stopSelf()
            return START_NOT_STICKY
        }
        if (!Settings.canDrawOverlays(this)) {
            stopSelf()
            return START_NOT_STICKY
        }
        if (overlay != null) {
            // Two tasks can share a reminder minute. Show the newest and drop
            // the older card: its alarm has already fired and marked itself,
            // so leaving both would stack overlays and strand a notification.
            runCatching { windowManager?.removeView(overlay) }
            NotificationManagerCompat.from(this).cancel(taskId.hashCode())
            overlay = null
        }
        taskId = id
        title = intent.getStringExtra("title") ?: "Task due"
        project = intent.getStringExtra("project")
        dueAt = intent.getLongExtra("due_at", System.currentTimeMillis())
        TaskReminders.markFired(this, id)
        startForeground(
            id.hashCode(),
            buildReminderNotification(this, id, title, project, dueAt),
        )
        showOverlay()
        if (!polling) {
            polling = true
            pollRemoteDismissal()
        }
        return START_NOT_STICKY
    }

    private fun showOverlay() {
        val preferences = PreferencesStore(this).load()
        if (preferences.pauseMediaOnReminder) {
            requestMediaPause()
        }
        val theme = ThemeStore(this).load()
        val systemDark = (resources.configuration.uiMode and
            Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
        val darkMode = when (theme.mode) {
            AppearanceMode.Dark -> true
            AppearanceMode.Light -> false
            AppearanceMode.System -> systemDark
        }
        val background = preferences.reminderBackgroundFor(theme, darkMode)
        val lightContent = darkMode || background is AppBackground.Custom
        val view = ComposeView(this).apply {
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setViewTreeLifecycleOwner(this@ReminderOverlayService)
            setViewTreeSavedStateRegistryOwner(this@ReminderOverlayService)
            setViewTreeViewModelStoreOwner(this@ReminderOverlayService)
            setContent {
                FlowlistTheme(
                    theme,
                    background = background,
                    backgroundScrim = if (background is AppBackground.Custom) {
                        androidx.compose.ui.graphics.Color.Black.copy(alpha = 0.45f)
                    } else {
                        androidx.compose.ui.graphics.Color.Transparent
                    },
                ) {
                    ReminderScreen(
                        title = title,
                        project = project,
                        dueAt = dueAt,
                        initialSnoozeMinutes = preferences.snoozeMinutes,
                        lightContent = lightContent,
                        onComplete = {
                            CoroutineScope(Dispatchers.IO).launch {
                                repository.dismissReminder(taskId)
                                repository.complete(taskId, true)
                            }
                            stopSelf()
                        },
                        onSnooze = { minutes ->
                            CoroutineScope(Dispatchers.IO).launch {
                                repository.dismissReminder(taskId)
                            }
                            TaskReminders.snooze(
                                this@ReminderOverlayService,
                                taskId,
                                title,
                                project,
                                minutes,
                            )
                            stopSelf()
                        },
                        onOpen = {
                            startActivity(
                                Intent(this@ReminderOverlayService, MainActivity::class.java).apply {
                                    flags = Intent.FLAG_ACTIVITY_NEW_TASK or
                                        Intent.FLAG_ACTIVITY_CLEAR_TOP
                                    putExtra("tab", "today")
                                },
                            )
                            stopSelf()
                        },
                        onDismiss = {
                            CoroutineScope(Dispatchers.IO).launch {
                                repository.dismissReminder(taskId)
                            }
                            stopSelf()
                        },
                    )
                }
            }
        }
        val manager = getSystemService(WindowManager::class.java) ?: run {
            stopSelf()
            return
        }
        val params = WindowManager.LayoutParams(
            WindowManager.LayoutParams.MATCH_PARENT,
            WindowManager.LayoutParams.MATCH_PARENT,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON,
            PixelFormat.TRANSLUCENT,
        ).apply {
            gravity = Gravity.TOP or Gravity.START
        }
        runCatching { manager.addView(view, params) }.onFailure {
            stopSelf()
            return
        }
        windowManager = manager
        overlay = view
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_START)
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_RESUME)
    }

    private fun pollRemoteDismissal() {
        scope.launch {
            while (isActive) {
                delay(1500)
                val dismissed = runCatching { repository.dismissedReminders() }
                    .getOrDefault(emptySet())
                if (taskId in dismissed) {
                    stopSelf()
                    break
                }
            }
        }
    }

    private val repository
        get() = (application as FlowlistApplication).repository

    /// Hold transient audio focus so a playing video or song pauses while the
    /// reminder owns the screen. Releasing it on close lets the app resume.
    private fun requestMediaPause() {
        val manager = getSystemService(AudioManager::class.java) ?: return
        val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT)
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build(),
            )
            .setOnAudioFocusChangeListener { }
            .build()
        if (manager.requestAudioFocus(request) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED) {
            audioFocusRequest = request
        }
    }

    override fun onDestroy() {
        // A foreground-service notification ignores a plain cancel(); drop the
        // foreground state first so the alarm card leaves the shade with the
        // reminder instead of lingering.
        runCatching { stopForeground(STOP_FOREGROUND_REMOVE) }
        scope.cancel()
        overlay?.let { view ->
            runCatching { windowManager?.removeView(view) }
        }
        overlay = null
        windowManager = null
        audioFocusRequest?.let {
            getSystemService(AudioManager::class.java)?.abandonAudioFocusRequest(it)
        }
        audioFocusRequest = null
        NotificationManagerCompat.from(this).cancel(taskId.hashCode())
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY)
        super.onDestroy()
    }
}
