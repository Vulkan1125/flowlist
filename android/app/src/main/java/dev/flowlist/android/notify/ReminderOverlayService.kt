package dev.flowlist.android.notify

import android.app.Service
import android.content.Intent
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.IBinder
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.view.WindowInsetsController
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
 *
 * Several tasks can come due in the same minute. Their alarms queue here and
 * each card is shown in turn, so one reminder never hides another.
 */
class ReminderOverlayService :
    Service(),
    LifecycleOwner,
    SavedStateRegistryOwner,
    ViewModelStoreOwner {
    private data class Pending(
        val id: String,
        val title: String,
        val project: String?,
        val dueAt: Long,
    )

    private val lifecycleRegistry = LifecycleRegistry(this)
    private val savedStateController = SavedStateRegistryController.create(this)
    private val store = ViewModelStore()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val pending = ArrayDeque<Pending>()
    private var current: Pending? = null
    private var windowManager: WindowManager? = null
    private var overlay: ComposeView? = null
    private var audioFocusRequest: AudioFocusRequest? = null
    private var polling = false

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
        val reminder = Pending(
            id = intent?.getStringExtra("task_id") ?: run {
                stopSelf()
                return START_NOT_STICKY
            },
            title = intent.getStringExtra("title") ?: "Task due",
            project = intent.getStringExtra("project"),
            dueAt = intent.getLongExtra("due_at", System.currentTimeMillis()),
        )
        if (!Settings.canDrawOverlays(this)) {
            stopSelf()
            return START_NOT_STICKY
        }
        TaskReminders.markFired(this, reminder.id)
        // The same alarm can arrive twice; keep one copy in the queue.
        if (current?.id == reminder.id || pending.any { it.id == reminder.id }) {
            return START_NOT_STICKY
        }
        if (current == null) {
            present(reminder)
        } else {
            pending.addLast(reminder)
        }
        return START_NOT_STICKY
    }

    /// Show one reminder. The freshness check runs first: the alarm comes from
    /// the last synced snapshot, so the Mac may have completed or snoozed this
    /// reminder after the phone last synced.
    private fun present(reminder: Pending) {
        current = reminder
        startForeground(
            reminder.id.hashCode(),
            buildReminderNotification(
                this,
                reminder.id,
                reminder.title,
                reminder.project,
                reminder.dueAt,
            ),
        )
        scope.launch {
            if (reminderAlreadyHandled(repository, reminder.id)) {
                finishCurrent()
                return@launch
            }
            if (current?.id != reminder.id) return@launch
            showOverlay(reminder)
            if (!polling) {
                polling = true
                pollRemoteDismissal()
            }
        }
    }

    /// Close the card on screen and move to the next task due at the same
    /// time, or stop once the queue is empty.
    private fun finishCurrent() {
        val done = current ?: return
        overlay?.let { view ->
            runCatching { windowManager?.removeView(view) }
        }
        overlay = null
        current = null
        NotificationManagerCompat.from(this).cancel(done.id.hashCode())
        val next = pending.removeFirstOrNull()
        if (next != null) {
            present(next)
        } else {
            stopSelf()
        }
    }

    private fun showOverlay(reminder: Pending) {
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
                        title = reminder.title,
                        project = reminder.project,
                        dueAt = reminder.dueAt,
                        initialSnoozeMinutes = preferences.snoozeMinutes,
                        lightContent = lightContent,
                        onComplete = {
                            CoroutineScope(Dispatchers.IO).launch {
                                repository.dismissReminder(reminder.id)
                                repository.complete(reminder.id, true)
                            }
                            finishCurrent()
                        },
                        onSnooze = { minutes ->
                            CoroutineScope(Dispatchers.IO).launch {
                                repository.dismissReminder(reminder.id)
                            }
                            TaskReminders.snooze(
                                this@ReminderOverlayService,
                                reminder.id,
                                reminder.title,
                                reminder.project,
                                minutes,
                            )
                            finishCurrent()
                        },
                        onOpen = {
                            startActivity(
                                Intent(this@ReminderOverlayService, MainActivity::class.java).apply {
                                    flags = Intent.FLAG_ACTIVITY_NEW_TASK or
                                        Intent.FLAG_ACTIVITY_CLEAR_TOP
                                    putExtra("tab", "today")
                                },
                            )
                            finishCurrent()
                        },
                        onDismiss = {
                            CoroutineScope(Dispatchers.IO).launch {
                                repository.dismissReminder(reminder.id)
                            }
                            finishCurrent()
                        },
                    )
                }
            }
        }
        val manager = getSystemService(WindowManager::class.java) ?: run {
            finishCurrent()
            return
        }
        val params = WindowManager.LayoutParams(
            WindowManager.LayoutParams.MATCH_PARENT,
            WindowManager.LayoutParams.MATCH_PARENT,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS or
                WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON,
            PixelFormat.TRANSLUCENT,
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                layoutInDisplayCutoutMode =
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                // Sit under the status and navigation bars so the theme
                // background reaches the edges, like the activity version.
                setFitInsetsTypes(0)
            }
        }
        runCatching { manager.addView(view, params) }.onFailure {
            finishCurrent()
            return
        }
        applySystemBarAppearance(view, lightContent)
        windowManager = manager
        overlay = view
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_START)
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_RESUME)
    }

    /// Match the status and navigation bar icons to the reminder's type, so
    /// they stay legible now that the window paints underneath them.
    private fun applySystemBarAppearance(view: View, lightContent: Boolean) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            view.windowInsetsController?.let { controller ->
                val mask = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
                    WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
                controller.setSystemBarsAppearance(if (lightContent) 0 else mask, mask)
            }
        } else {
            @Suppress("DEPRECATION")
            view.systemUiVisibility = if (lightContent) {
                0
            } else {
                View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR or
                    View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
            }
        }
    }

    private fun pollRemoteDismissal() {
        scope.launch {
            while (isActive) {
                delay(1500)
                val active = current ?: continue
                val dismissed = runCatching { repository.dismissedReminders() }
                    .getOrDefault(emptySet())
                if (active.id in dismissed) finishCurrent()
            }
        }
    }

    private val repository
        get() = (application as FlowlistApplication).repository

    /// Hold transient audio focus so a playing video or song pauses while the
    /// reminder owns the screen. Releasing it on close lets the app resume.
    private fun requestMediaPause() {
        if (audioFocusRequest != null) return
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
        current?.let { NotificationManagerCompat.from(this).cancel(it.id.hashCode()) }
        current = null
        pending.clear()
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY)
        super.onDestroy()
    }
}
