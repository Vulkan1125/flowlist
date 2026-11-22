package dev.flowlist.android

import dev.flowlist.android.data.parsePairingQr
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PairingTest {
    @Test
    fun parsesQrWithEveryAddress() {
        val scan = parsePairingQr(
            "flowlist://pair?code=123456&address=192.168.1.5:7777&address=100.115.75.15:7777",
        )
        assertEquals("123456", scan?.code)
        assertEquals(
            listOf("http://192.168.1.5:7777", "http://100.115.75.15:7777"),
            scan?.addresses,
        )
    }

    @Test
    fun keepsAnExplicitScheme() {
        val scan = parsePairingQr(
            "flowlist://pair?code=654321&address=http%3A%2F%2F10.0.0.2%3A7777",
        )
        assertEquals(listOf("http://10.0.0.2:7777"), scan?.addresses)
    }

    @Test
    fun rejectsForeignLinks() {
        assertNull(parsePairingQr("https://example.com/pair?code=123456&address=1.2.3.4:7777"))
        assertNull(parsePairingQr("flowlist://task/1234"))
        assertNull(parsePairingQr("flowlist://pair?code=12&address=1.2.3.4:7777"))
        assertNull(parsePairingQr("not a uri"))
    }
}
