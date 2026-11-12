package com.crosslink.app.service

import android.inputmethodservice.InputMethodService
import android.os.Bundle
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.util.Log

class CrossLinkIME : InputMethodService() {
    companion object {
        private const val TAG = "CrossLinkIME"
        var instance: CrossLinkIME? = null
        var activeInputConnection: InputConnection? = null
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        Log.i(TAG, "IME service created")
    }

    override fun onStartInput(editorInfo: EditorInfo, restarting: Boolean) {
        super.onStartInput(editorInfo, restarting)
        activeInputConnection = currentInputConnection
        Log.d(TAG, "IME started input: restarting=$restarting")
    }

    override fun onFinishInput() {
        super.onFinishInput()
        activeInputConnection = null
        Log.d(TAG, "IME finished input")
    }

    fun commitText(text: String) {
        val conn = activeInputConnection ?: currentInputConnection ?: return
        conn.commitText(text, 0, null)
        conn.sendKeyEvent(null)
    }

    fun deleteBackspace(count: Int = 1) {
        val conn = activeInputConnection ?: currentInputConnection ?: return
        repeat(count) {
            conn.sendKeyEvent(
                android.view.KeyEvent(
                    0,
                    android.view.KeyCharacterMap.empty
                )
            )
        }
    }

    fun performEnter() {
        val conn = activeInputConnection ?: currentInputConnection ?: return
        conn.sendKeyEvent(
            android.view.KeyEvent(
                0,
                android.view.KeyCharacterMap.empty
            )
        )
    }

    override fun onDestroy() {
        instance = null
        super.onDestroy()
        Log.i(TAG, "IME service destroyed")
    }
}