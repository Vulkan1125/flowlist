package dev.flowlist.android.notify

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.google.android.gms.location.Geofence
import com.google.android.gms.location.GeofencingClient
import com.google.android.gms.location.GeofencingRequest
import com.google.android.gms.location.LocationServices
import dev.flowlist.android.MainActivity
import dev.flowlist.android.R
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.FileStore
import dev.flowlist.android.data.tasksForTag

/** Registers a geofence per geotagged tag and posts a reminder on arrival. */
object NearbyReminders {
    const val ACTION = "dev.flowlist.android.NEARBY"
    const val CHANNEL = "nearby"
    private const val PREFS = "flowlist.notifications"
    private const val ENABLED = "nearby"
    private const val RADIUS_METERS = 250f

    fun enabled(context: Context): Boolean =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(ENABLED, true)

    fun setEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(ENABLED, enabled)
            .apply()
        if (!enabled) {
            runCatching { client(context).removeGeofences(pendingIntent(context)) }
        }
    }

    fun canPost(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED

    fun ensureChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val channel = NotificationChannel(
            CHANNEL,
            "Nearby reminders",
            NotificationManager.IMPORTANCE_DEFAULT,
        ).apply {
            description = "A reminder when you arrive near a geotagged place."
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    /** Rebuilds the geofence set from the cached library. Safe to call often. */
    fun sync(context: Context, db: Db) {
        if (!enabled(context) || !canPost(context)) return
        if (!DeviceLocation.hasForeground(context) || !DeviceLocation.hasBackground(context)) return
        val client = client(context)
        val pending = pendingIntent(context)
        client.removeGeofences(pending)
        val fences = db.tags.values.mapNotNull { tag ->
            val place = tag.location ?: return@mapNotNull null
            if (db.tasksForTag(tag.id).isEmpty()) return@mapNotNull null
            Geofence.Builder()
                .setRequestId(tag.id)
                .setCircularRegion(place.lat, place.lon, RADIUS_METERS)
                .setExpirationDuration(Geofence.NEVER_EXPIRE)
                .setTransitionTypes(Geofence.GEOFENCE_TRANSITION_ENTER)
                .build()
        }
        if (fences.isEmpty()) return
        val request = GeofencingRequest.Builder()
            .setInitialTrigger(GeofencingRequest.INITIAL_TRIGGER_ENTER)
            .addGeofences(fences)
            .build()
        runCatching { client.addGeofences(request, pending) }
    }

    fun notifyArrival(context: Context, tagId: String) {
        if (!canPost(context)) return
        val snapshot = FileStore(context).loadSnapshot() ?: return
        val tag = snapshot.db.tags[tagId] ?: return
        val tasks = snapshot.db.tasksForTag(tag.id)
        if (tasks.isEmpty()) return
        ensureChannel(context)
        val title = "Near #${tag.name}"
        val details = tasks.take(4).joinToString("\n") { "• ${it.title}" }
        val summary = if (tasks.size == 1) "1 task here" else "${tasks.size} tasks here"
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("more", "nearby")
        }
        val pending = PendingIntent.getActivity(
            context,
            tagId.hashCode(),
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_flowlist)
            .setContentTitle(title)
            .setContentText(summary)
            .setStyle(NotificationCompat.BigTextStyle().bigText("$summary\n$details"))
            .setContentIntent(pending)
            .setAutoCancel(true)
            .setOnlyAlertOnce(true)
            .build()
        NotificationManagerCompat.from(context).notify(tagId.hashCode(), notification)
    }

    fun sendTest(context: Context) {
        if (!canPost(context)) return
        ensureChannel(context)
        val snapshot = FileStore(context).loadSnapshot()
        val tag = snapshot?.db?.tags?.values?.firstOrNull { tag ->
            tag.location != null && snapshot.db.tasksForTag(tag.id).isNotEmpty()
        }
        if (tag != null) {
            notifyArrival(context, tag.id)
            return
        }
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("more", "nearby")
        }
        val pending = PendingIntent.getActivity(
            context,
            21,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_flowlist)
            .setContentTitle("Nearby reminders are on")
            .setContentText("You will hear about tasks when you arrive near a geotagged tag.")
            .setContentIntent(pending)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(21, notification)
    }

    private fun client(context: Context): GeofencingClient =
        LocationServices.getGeofencingClient(context)

    private fun pendingIntent(context: Context): PendingIntent =
        PendingIntent.getBroadcast(
            context,
            20,
            Intent(context, NearbyReceiver::class.java).setAction(ACTION),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
        )
}
