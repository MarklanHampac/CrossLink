package com.crosslink.app.net

import android.util.Log
import com.crosslink.app.protocol.InputPacket
import com.crosslink.app.security.Crypto
import com.crosslink.app.service.CrossLinkAccessibilityService
import com.crosslink.app.service.CrossLinkIME
import com.crosslink.app.ui.CursorOverlay
import kotlinx.coroutines.*
import java.io.DataInputStream
import java.io.DataOutputStream
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean
import javax.crypto.SecretKey

class InputReceiverServer(
    private val port: Int = 8888,
    private val onStatus: (String) -> Unit = {}
) {
    companion object {
        private const val TAG = "InputReceiverServer"
        private val MAGIC = byteArrayOf('C'.code.toByte(), 'R'.code.toByte(), 'L'.code.toByte(), 'K'.code.toByte())
        private const val VER: Byte = 1
        private const val MAX_FRAME = 64 * 1024
    }

    private var serverSocket: ServerSocket? = null
    private val running = AtomicBoolean(false)
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    @Volatile var peerName: String? = null
    @Volatile var lastCursorX: Float = 0f
    @Volatile var lastCursorY: Float = 0f

    fun start() {
        if (!running.compareAndSet(false, true)) return
        scope.launch {
            try {
                serverSocket = java.net.ServerSocket().apply {
                    reuseAddress = true
                    bind(java.net.InetSocketAddress("0.0.0.0", port))
                }
                Log.i(TAG, "Server started on $port")
                onStatus("Listening on 0.0.0.0:$port")
                while (running.get()) {
                    val client = try { serverSocket?.accept() } catch (e: Exception) {
                        if (running.get()) onStatus("Accept error: ${e.message}")
                        null
                    } ?: if (running.get()) continue else break
                    val addr = try { client.inetAddress?.hostAddress ?: "?" } catch (_: Exception) { "?" }
                    Log.i(TAG, "Client connected: $addr")
                    onStatus("Client connected: $addr")
                    handleClient(client)
                    if (running.get()) onStatus("Listening on 0.0.0.0:$port")
                }
            } catch (e: Exception) {
                Log.e(TAG, "Server error", e)
                onStatus("Server error: ${e.message}. Check app permissions/network and that port $port is free.")
            } finally {
                if (running.get()) onStatus("Server stopped")
            }
        }
    }

    private suspend fun handleClient(socket: Socket) = withContext(Dispatchers.IO) {
        var out: DataOutputStream? = null
        try {
            socket.tcpNoDelay = true
            socket.soTimeout = 15000
            val input = DataInputStream(socket.getInputStream())
            out = DataOutputStream(socket.getOutputStream())

            val magic = ByteArray(4); input.readFully(magic)
            if (!magic.contentEquals(MAGIC)) { Log.w(TAG, "bad magic"); return@withContext }
            val ver = input.readByte()
            val nlen = input.readByte().toInt() and 0xFF
            val nameBytes = ByteArray(nlen); input.readFully(nameBytes)
            val clientName = String(nameBytes, Charsets.UTF_8)
            val peerPub = ByteArray(32); input.readFully(peerPub)

            val myPub = Crypto.randomPub()
            synchronized(out) {
                out.write(MAGIC); out.writeByte(VER.toInt())
                val myName = "CrossLink-Android".toByteArray(Charsets.UTF_8)
                out.writeByte(myName.size); out.write(myName); out.write(myPub); out.flush()
            }

            val key = Crypto.deriveKey(myPub, peerPub)
            peerName = clientName
            onStatus("Connected: $clientName")
            sendPacket(out, key, InputPacket.HandshakeResponse(true, "CrossLink-Android", "paired"))

            socket.soTimeout = 1000
            var lastRecv = System.currentTimeMillis()
            while (running.get() && !socket.isClosed) {
                if (System.currentTimeMillis() - lastRecv > 15000) { Log.w(TAG, "timeout"); break }
                try {
                    val len = input.readInt()
                    if (len <= 0 || len > MAX_FRAME) break
                    val frame = ByteArray(len); input.readFully(frame)
                    val plain = Crypto.decrypt(key, frame)
                    val pkt = InputPacket.fromJson(String(plain, Charsets.UTF_8)) ?: continue
                    lastRecv = System.currentTimeMillis()
                    handlePacket(pkt, out, key)
                } catch (e: java.net.SocketTimeoutException) {
                    // keepalive window
                } catch (e: Exception) {
                    Log.w(TAG, "recv err: ${e.message}")
                    break
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "client error", e)
        } finally {
            try { socket.close() } catch (_: Exception) {}
            peerName = null
            onStatus("Client disconnected. Listening on 0.0.0.0:$port")
        }
    }

    private fun sendPacket(out: DataOutputStream, key: SecretKey, pkt: InputPacket) {
        try {
            val plain = pkt.toJson().toByteArray(Charsets.UTF_8)
            val frame = Crypto.encrypt(key, plain)
            synchronized(out) { out.writeInt(frame.size); out.write(frame); out.flush() }
        } catch (e: Exception) { Log.w(TAG, "send err: ${e.message}") }
    }

    private fun handlePacket(pkt: InputPacket, out: DataOutputStream, key: SecretKey) {
        when (pkt) {
            is InputPacket.MouseMove -> {
                val x = if (pkt.screenWidth > 0) pkt.x.toFloat() / pkt.screenWidth * displayWidth() else pkt.x.toFloat()
                val y = if (pkt.screenHeight > 0) pkt.y.toFloat() / pkt.screenHeight * displayHeight() else pkt.y.toFloat()
                lastCursorX = x; lastCursorY = y
                CursorOverlay.update(x, y)
            }
            is InputPacket.MouseClick -> {
                if (pkt.action == "down" || pkt.action == "click") {
                    CrossLinkAccessibilityService.instance?.performTap(lastCursorX, lastCursorY)
                }
            }
            is InputPacket.MouseScroll -> {
                val x = lastCursorX; val y = lastCursorY
                val dy = (pkt.deltaY / 2).coerceIn(-400, 400)
                CrossLinkAccessibilityService.instance?.performScroll(x, y - dy, x, y + dy, 120)
            }
            is InputPacket.KeyEvent -> {
                if (pkt.action == "down") {
                    val ch = pkt.unicodeChar
                    if (ch != null && ch.isNotEmpty() && ch[0].code >= 32) {
                        CrossLinkIME.instance?.commitText(ch)
                    } else when (pkt.vkCode) {
                        0x08 -> CrossLinkIME.instance?.deleteBackspace()
                        0x0D -> CrossLinkIME.instance?.performEnter()
                        0x09 -> CrossLinkIME.instance?.commitText("\t")
                        0x20 -> CrossLinkIME.instance?.commitText(" ")
                        0x1B -> { /* handled on host as return */ }
                    }
                }
            }
            is InputPacket.TextCommit -> CrossLinkIME.instance?.commitText(pkt.text)
            is InputPacket.Heartbeat -> sendPacket(out, key, InputPacket.Heartbeat(System.currentTimeMillis()))
            is InputPacket.Handshake -> sendPacket(out, key, InputPacket.HandshakeResponse(true, "CrossLink-Android", "ok"))
            else -> {}
        }
    }

    private fun displayWidth(): Float = android.content.res.Resources.getSystem().displayMetrics.widthPixels.toFloat()
    private fun displayHeight(): Float = android.content.res.Resources.getSystem().displayMetrics.heightPixels.toFloat()

    fun stop() {
        running.set(false)
        try { serverSocket?.close() } catch (_: Exception) {}
        scope.cancel()
    }
}
