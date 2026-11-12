package com.crosslink.app.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.accessibilityservice.GestureDescription
import android.content.Intent
import android.graphics.Path
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo

class CrossLinkAccessibilityService : AccessibilityService() {
    companion object {
        private const val TAG = "CrossLinkAccessibility"
        var instance: CrossLinkAccessibilityService? = null
        // Global action constants (from AccessibilityService API)
        private const val GLOBAL_ACTION_OPEN_NOTIFICATIONS = 6
        private const val GLOBAL_ACTION_POWER_MENU = 12
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
            feedbackType = AccessibilityServiceInfo.FEEDBACK_ALL_MASK
            notificationTimeout = 100
            flags = AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS or
                    AccessibilityServiceInfo.FLAG_REQUEST_TOUCH_EXPLORATION_MODE
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
        val builder = GestureDescription.Builder()
        builder.addStroke(GestureDescription.StrokeDescription(path, 0, 100))
        val gesture = builder.build()
        val callbackObj = object : AccessibilityService.GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription) {
                callback?.invoke(true)
            }

            override fun onCancelled(gestureDescription: GestureDescription) {
                callback?.invoke(false)
            }
        }
        dispatchGesture(gesture, callbackObj, null)
    }

    fun performScroll(startX: Float, startY: Float, endX: Float, endY: Float, duration: Long = 200, callback: ((Boolean) -> Unit)? = null) {
        val path = Path().apply {
            moveTo(startX, startY)
            lineTo(endX, endY)
        }
        val builder = GestureDescription.Builder()
        builder.addStroke(GestureDescription.StrokeDescription(path, 0, duration))
        val gesture = builder.build()
        val callbackObj = object : AccessibilityService.GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription) {
                callback?.invoke(true)
            }

            override fun onCancelled(gestureDescription: GestureDescription) {
                callback?.invoke(false)
            }
        }
        dispatchGesture(gesture, callbackObj, null)
    }

    fun performLongPress(x: Float, y: Float, duration: Long = 500, callback: ((Boolean) -> Unit)? = null) {
        val path = Path().apply { moveTo(x, y) }
        val builder = GestureDescription.Builder()
        builder.addStroke(GestureDescription.StrokeDescription(path, 0, duration))
        val gesture = builder.build()
        val callbackObj = object : AccessibilityService.GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription) {
                callback?.invoke(true)
            }

            override fun onCancelled(gestureDescription: GestureDescription) {
                callback?.invoke(false)
            }
        }
        dispatchGesture(gesture, callbackObj, null)
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
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN) {
            performGlobalAction(GLOBAL_ACTION_OPEN_NOTIFICATIONS)
        }
    }

    fun performPowerDialog() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            performGlobalAction(GLOBAL_ACTION_POWER_MENU)
        }
    }
}