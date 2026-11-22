package dev.flowlist.android.data

import java.net.URI
import java.net.URLDecoder

/** What a pairing QR carries: every reachable Mac address plus the six-digit code. */
data class PairingScan(val addresses: List<String>, val code: String)

/**
 * Parses `flowlist://pair?code=123456&address=192.168.1.5:7777&address=...`.
 * The Mac repeats `address` for each interface so the phone can try the LAN
 * first and Tailscale second. Anything else is not a pairing code.
 */
fun parsePairingQr(raw: String): PairingScan? {
    val uri = runCatching { URI(raw.trim()) }.getOrNull() ?: return null
    if (!uri.scheme.equals("flowlist", ignoreCase = true) || uri.host != "pair") return null
    val query = uri.rawQuery ?: return null
    val params = query.split('&').mapNotNull { part ->
        val split = part.indexOf('=')
        if (split <= 0) {
            null
        } else {
            val key = URLDecoder.decode(part.substring(0, split), "UTF-8")
            val value = URLDecoder.decode(part.substring(split + 1), "UTF-8")
            key to value
        }
    }
    val code = params.firstOrNull { it.first == "code" }?.second?.trim().orEmpty()
    val addresses = params
        .filter { it.first == "address" }
        .map { it.second.trim().trimEnd('/') }
        .filter { it.isNotEmpty() }
        .map { value ->
            if (value.startsWith("http://") || value.startsWith("https://")) value else "http://$value"
        }
        .distinct()
    if (code.length < 4 || addresses.isEmpty()) return null
    return PairingScan(addresses, code)
}
