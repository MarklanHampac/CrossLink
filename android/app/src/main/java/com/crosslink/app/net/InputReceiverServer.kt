package com.crosslink.app.net

import android.util.Log
import com.crosslink.app.service.CrossLinkAccessibilityService
import com.crosslink.app.service.CrossLinkIME
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import java.io.InputStream
import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.nio.ByteOrder

class InputReceiverServer(private val port: Int = 8888) {
    companion object {
        private const val TAG = "InputReceiverServer"
    }

    private var serverSocket: ServerSocket? = null
    private var isRunning = false

    fun start() {
        isRunning = true
        GlobalScope.launch(Dispatchers.IO) {
            try {
                serverSocket = ServerSocket(port)
                Log.i(TAG, "InputReceiverServer started on port $port")
                while (isRunning) {
                    val clientSocket = serverSocket?.accept()
                    clientSocket?.let { handleClient(it) }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Server error: ${e.message}")
            }
        }
    }

    private fun handleClient(socket: Socket) {
        GlobalScope.launch(Dispatchers.IO) {
            try {
                val input: InputStream = socket.getInputStream()
                val buffer = ByteArray(1024)
                while (isRunning && !socket.isClosed) {
                    val bytesRead = input.read(buffer)
                    if (bytesRead <= 0) break
                    processPacket(buffer, bytesRead)
                }
            } catch (e: Exception) {
                Log.e(TAG, "Client socket error: ${e.message}")
            } finally {
                try { socket.close() } catch (_: Exception) {}
            }
        }
    }

    private fun processPacket(buffer: ByteArray, length: Int) {
        if (length < 4) return
        val type = buffer[0].toInt()
        when (type) {
            0x01 -> { // Pointer Move
                val bb = ByteBuffer.wrap(buffer, 1, 8).order(ByteOrder.LITTLE_ENDIAN)
                val x = bb.int.toFloat()
                val y = bb.int.toFloat()
                // Update Overlay Position
            }
            0x02 -> { // Tap / Click
                val bb = ByteBuffer.wrap(buffer, 1, 8).order(ByteOrder.LITTLE_ENDIAN)
                val x = bb.int.toFloat()
                val y = bb.int.toFloat()
                CrossLinkAccessibilityService.instance?.performTap(x, y)
            }
            0x03 -> { // Scroll
                val bb = ByteBuffer.wrap(buffer, 1, 16).order(ByteOrder.LITTLE_ENDIAN)
                val sx = bb.int.toFloat()
                val sy = bb.int.toFloat()
                val ex = bb.int.toFloat()
                val ey = bb.int.toFloat()
                CrossLinkAccessibilityService.instance?.performScroll(sx, sy, ex, ey, 200)
            }
            0x04 -> { // Text
                val text = String(buffer, 1, length - 1, Charsets.UTF_8)
                CrossLinkIME.instance?.commitText(text)
            }
            0x05 -> { // Key Event (Backspace / Enter / Back)
                val key = buffer[1].toInt()
                if (key == 0x08) {
                    CrossLinkIME.instance?.deleteBackspace()
                } else if (key == 0x0D) {
                    CrossLinkIME.instance?.performEnter()
                } else if (key == 0x1B) { // ESC -> Back
                    CrossLinkAccessibilityService.instance?.performBack()
                }
            }
        }
    }

    fun stop() {
        isRunning = false
        try { serverSocket?.close() } catch (_: Exception) {}
    }
}