package com.crosslink.app.protocol

import org.json.JSONObject

sealed class InputPacket {
    data class MouseMove(val x: Int, val y: Int, val dx: Int, val dy: Int, val screenWidth: Int, val screenHeight: Int) : InputPacket()
    data class MouseClick(val button: String, val action: String, val x: Int, val y: Int) : InputPacket()
    data class MouseScroll(val deltaX: Int, val deltaY: Int) : InputPacket()
    data class KeyEvent(val vkCode: Int, val scanCode: Int, val action: String, val unicodeChar: String?, val ctrl: Boolean, val alt: Boolean, val shift: Boolean, val meta: Boolean) : InputPacket()
    data class TextCommit(val text: String) : InputPacket()
    data class Handshake(val deviceName: String, val version: String) : InputPacket()
    data class HandshakeResponse(val accepted: Boolean, val deviceName: String, val message: String) : InputPacket()
    data class Heartbeat(val timestamp: Long) : InputPacket()
    object ReturnControl : InputPacket()

    fun toJson(): String {
        val o = JSONObject()
        when (this) {
            is MouseMove -> { o.put("t","mouse_move"); o.put("x",x); o.put("y",y); o.put("dx",dx); o.put("dy",dy); o.put("screen_width",screenWidth); o.put("screen_height",screenHeight) }
            is MouseClick -> { o.put("t","mouse_click"); o.put("button",button); o.put("action",action); o.put("x",x); o.put("y",y) }
            is MouseScroll -> { o.put("t","mouse_scroll"); o.put("delta_x",deltaX); o.put("delta_y",deltaY) }
            is KeyEvent -> { o.put("t","key_event"); o.put("vk_code",vkCode); o.put("scan_code",scanCode); o.put("action",action); if (unicodeChar!=null) o.put("unicode_char",unicodeChar); o.put("ctrl",ctrl); o.put("alt",alt); o.put("shift",shift); o.put("meta",meta) }
            is TextCommit -> { o.put("t","text_commit"); o.put("text",text) }
            is Handshake -> { o.put("t","handshake"); o.put("device_name",deviceName); o.put("version",version) }
            is HandshakeResponse -> { o.put("t","handshake_response"); o.put("accepted",accepted); o.put("device_name",deviceName); o.put("message",message) }
            is Heartbeat -> { o.put("t","heartbeat"); o.put("timestamp",timestamp) }
            ReturnControl -> { o.put("t","return_control") }
        }
        return o.toString()
    }

    companion object {
        fun fromJson(s: String): InputPacket? = try {
            val o = JSONObject(s)
            when (o.optString("t")) {
                "mouse_move" -> MouseMove(o.optInt("x"), o.optInt("y"), o.optInt("dx"), o.optInt("dy"), o.optInt("screen_width"), o.optInt("screen_height"))
                "mouse_click" -> MouseClick(o.optString("button"), o.optString("action"), o.optInt("x"), o.optInt("y"))
                "mouse_scroll" -> MouseScroll(o.optInt("delta_x"), o.optInt("delta_y"))
                "key_event" -> KeyEvent(o.optInt("vk_code"), o.optInt("scan_code"), o.optString("action"), if (o.has("unicode_char") && !o.isNull("unicode_char")) o.optString("unicode_char") else null, o.optBoolean("ctrl"), o.optBoolean("alt"), o.optBoolean("shift"), o.optBoolean("meta"))
                "text_commit" -> TextCommit(o.optString("text"))
                "handshake" -> Handshake(o.optString("device_name"), o.optString("version"))
                "handshake_response" -> HandshakeResponse(o.optBoolean("accepted"), o.optString("device_name"), o.optString("message"))
                "heartbeat" -> Heartbeat(o.optLong("timestamp"))
                "return_control" -> ReturnControl
                else -> null
            }
        } catch (_: Exception) { null }
    }
}
