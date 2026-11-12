package com.crosslink.app

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.accessibilityservice.GestureResult
import android.content.Intent
import android.graphics.Path
import android.os.Bundle
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.util.Log
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class CrossLinkAccessibilityService : AccessibilityService() {
    companion object {
        private const val TAG = "CrossLinkAccessibility"
        var instance: CrossLinkAccessibilityService? = null
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Handle accessibility events if needed
    }

    override fun onInterrupt() {
        Log.w(TAG, "Accessibility service interrupted")
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        val info = AccessibilityServiceInfo().apply {
            eventTypes = AccessibilityEvent.TYPE_VIEW_CLICKED or
                    AccessibilityEvent.TYPE_VIEW_LONG_CLICKED or
                    AccessibilityEvent.TYPE_VIEW_FOCUSED or
                    AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED or
                    AccessibilityEvent.TYPE_NOTIFICATION_STATE_CHANGED or
                    AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED
            feedbackType = AccessibilityServiceInfo.FEEDBACK_ALL_MASK or AccessibilityServiceInfo.FEEDBACK_SPOKEN
            notificationTimeout = 100
            flags = AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS or
                    AccessibilityServiceInfo.FLAG_INCLUDE_NOT_TOUCHABLE or
                    AccessibilityServiceInfo.FLAG_RETRIEVE_WINDOW_CONTENT or
                    AccessibilityServiceInfo.FLAG_ENABLE_ACCESSIBILITY_API or
                    AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS or
                    AccessibilityServiceInfo.FLAG_REQUEST_TOUCH_EXPLORATION_MODE or
                    AccessibilityServiceInfo.FLAG_REQUEST_EXPLORATION_MODE or
                    AccessibilityServiceInfo.FLAG_REQUEST_STYLUS_EXPLORATION_MODE or
                    AccessibilityServiceInfo.FLAG_REQUEST_2D_ICON_SUPPORT or
                    AccessibilityServiceInfo.FLAG_RETRIEVE_HARDWARE_TEXT
        }
        serviceInfo = info
        Log.i(TAG, "Accessibility service connected")
    }

    override fun onUnbind(intent: Intent?): Boolean {
        instance = null
        return super.onUnbind(intent)
    }

    fun performTap(x: Float, y: Float, callback: ((Boolean) -> Unit)? = null) {
        val path = Path().apply { moveTo(x, y) }
        val gesture = GestureResult.Builder()
            .addStroke(GestureResult.Stroke(path, 0, 100))
            .build()
        val latch = CountDownLatch(1)
        var success = false

        val gestureCallback = object : GestureResult.Callback() {
            override fun onCompleted(gestureResult: GestureResult) {
                success = true
                latch.countDown()
                callback?.invoke(true)
            }

            override fun onFailed(gestureResult: GestureResult) {
                latch.countDown()
                callback?.invoke(false)
            }
        }

        dispatchGesture(gesture, gestureCallback, null)
        try {
            latch.await(2, TimeUnit.SECONDS)
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            callback?.invoke(false)
        }
    }

    fun performScroll(startX: Float, startY: Float, endX: Float, endY: Float, duration: Long, callback: ((Boolean) -> Unit)? = null) {
        val path = Path().apply {
            moveTo(startX, startY)
            lineTo(endX, endY)
        }
        val gesture = GestureResult.Builder()
            .addStroke(GestureResult.Stroke(path, 0, duration))
            .build()
        val latch = CountDownLatch(1)
        var success = false

        val gestureCallback = object : GestureResult.Callback() {
            override fun onCompleted(gestureResult: GestureResult) {
                success = true
                latch.countDown()
                callback?.invoke(true)
            }

            override fun onFailed(gestureResult: GestureResult) {
                latch.countDown()
                callback?.invoke(false)
            }
        }

        dispatchGesture(gesture, gestureCallback, null)
        try {
            latch.await(2, TimeUnit.SECONDS)
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            callback?.invoke(false)
        }
    }

    fun performLongPress(x: Float, y: Float, duration: Long = 500, callback: ((Boolean) -> Unit)? = null) {
        val path = Path().apply { moveTo(x, y) }
        val gesture = GestureResult.Builder()
            .addStroke(GestureResult.Stroke(path, 0, duration))
            .build()
        val latch = CountDownLatch(1)
        var success = false

        val gestureCallback = object : GestureResult.Callback() {
            override fun onCompleted(gestureResult: GestureResult) {
                success = true
                latch.countDown()
                callback?.invoke(true)
            }

            override fun onFailed(gestureResult: GestureResult) {
                latch.countDown()
                callback?.invoke(false)
            }
        }

        dispatchGesture(gesture, gestureCallback, null)
        try {
            latch.await(2, TimeUnit.SECONDS)
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            callback?.invoke(false)
        }
    }

    fun performBack() {
        performGlobalAction(GLOBAL_ACTION_BACK)
    }

    fun performHome() {
        performGlobalAction(GLOBAL_ACTION_HOME)
    }

    fun performRecents() {
        performGlobalAction(GLOBAL_ACTION_RECENTS)
    }

    fun performOpenNotifications() {
        performGlobalAction(GLOBAL_ACTION_OPEN_NOTIFICATIONS)
    }

    fun performPowerDialog() {
        performGlobalAction(GLOBAL_ACTION_POWER_MENU)
    }
}