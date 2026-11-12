package com.crosslink.app.net

import android.util.Log
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.util.concurrent.atomic.AtomicBoolean

class DiscoveryResponder(private val port: Int = 8889) {
    companion object { private const val TAG = "DiscoveryResponder" }
    private val running = AtomicBoolean(false)
    private var socket: DatagramSocket? = null
    private var thread: Thread? = null

    fun start() {
        if (!running.compareAndSet(false, true)) return
        thread = Thread {
            try {
                val s = DatagramSocket(port, InetAddress.getByName("0.0.0.0"))
                s.broadcast = true
                socket = s
                val buf = ByteArray(512)
                while (running.get()) {
                    val pkt = DatagramPacket(buf, buf.size)
                    try {
                        s.receive(pkt)
                        val msg = String(pkt.data, 0, pkt.length, Charsets.UTF_8)
                        if (msg.trim() == "CROSSLINK_DISCOVER") {
                            val resp = "CROSSLINK_HERE:8888".toByteArray(Charsets.UTF_8)
                            val out = DatagramPacket(resp, resp.size, pkt.address, pkt.port)
                            s.send(out)
                            Log.i(TAG, "Discovery replied to ${pkt.address.hostAddress}")
                        }
                    } catch (_: Exception) { if (!running.get()) break }
                }
            } catch (e: Exception) { Log.e(TAG, "discovery error", e) }
        }.also { it.isDaemon = true; it.start() }
    }

    fun stop() {
        running.set(false)
        try { socket?.close() } catch (_: Exception) {}
    }
}
