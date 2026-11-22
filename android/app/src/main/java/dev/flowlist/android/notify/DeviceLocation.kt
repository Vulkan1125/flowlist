package dev.flowlist.android.notify

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import com.google.android.gms.location.FusedLocationProviderClient
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import com.google.android.gms.tasks.CancellationTokenSource
import dev.flowlist.android.data.GeoDto
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/** Fine/coarse location plus the background grant geofences need. */
object DeviceLocation {
    fun hasForeground(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_COARSE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED

    fun hasBackground(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.Q ||
            ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            ) == PackageManager.PERMISSION_GRANTED

    suspend fun current(context: Context): GeoDto? {
        if (!hasForeground(context)) return null
        val client = LocationServices.getFusedLocationProviderClient(context)
        last(client)?.let { return it }
        return suspendCancellableCoroutine { continuation ->
            val token = CancellationTokenSource()
            continuation.invokeOnCancellation { token.cancel() }
            runCatching {
                client.getCurrentLocation(Priority.PRIORITY_HIGH_ACCURACY, token.token)
            }.getOrNull()
                ?.addOnSuccessListener { location ->
                    continuation.resume(
                        location?.let { GeoDto(it.latitude, it.longitude) },
                    )
                }
                ?.addOnFailureListener { continuation.resume(null) }
                ?: continuation.resume(null)
        }
    }

    private suspend fun last(client: FusedLocationProviderClient): GeoDto? =
        suspendCancellableCoroutine { continuation ->
            runCatching { client.lastLocation }
                .getOrNull()
                ?.addOnSuccessListener { location ->
                    continuation.resume(
                        location?.let { GeoDto(it.latitude, it.longitude) },
                    )
                }
                ?.addOnFailureListener { continuation.resume(null) }
                ?: continuation.resume(null)
        }
}
