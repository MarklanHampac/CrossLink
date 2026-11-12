package com.crosslink.app.ui

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.WindowManager
import android.widget.FrameLayout

class CursorOverlay(context: Context) : FrameLayout(context) {
    var cursorX: Float = 0f
        set(value) {
            field = value
            updatePosition()
        }
    var cursorY: Float = 0f
        set(value) {
            field = value
            updatePosition()
        }

    init {
        isFocusable = false
        isFocusableInTouchMode = false
        setWillNotDraw(false)
    }

    private fun updatePosition() {
        val params = layoutParams as? WindowManager.LayoutParams
        if (params != null) {
            params.x = cursorX.toInt()
            params.y = cursorY.toInt()
            val wm = context.getSystemService(Context.WINDOW_SERVICE) as? WindowManager
            wm?.updateViewLayout(this, params)
        }
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val paint = Paint().apply {
            color = Color.argb(200, 255, 255, 255)
            style = Paint.Style.STROKE
            strokeWidth = 4f
            isAntiAlias = true
        }
        canvas.drawCircle(cursorX, cursorY, 12f, paint)
    }
}