package com.crosslink.app.ui

import android.content.Context
import android.graphics.PixelFormat
import android.os.Build
import android.provider.Settings
import android.view.Gravity
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.ImageView

object CursorOverlay {
    private var view: ImageView? = null
    private var wm: WindowManager? = null
    private var params: WindowManager.LayoutParams? = null
    @Volatile var visible: Boolean = false

    fun show(ctx: Context) {
        if (view != null) return
        if (!Settings.canDrawOverlays(ctx)) return
        val wmgr = ctx.getSystemService(Context.WINDOW_SERVICE) as WindowManager
        val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O)
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        else @Suppress("DEPRECATION") WindowManager.LayoutParams.TYPE_PHONE
        val p = WindowManager.LayoutParams(
            40, 40, type,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        )
        p.gravity = Gravity.TOP or Gravity.START
        val iv = ImageView(ctx)
        iv.setImageResource(android.R.drawable.presence_online)
        iv.alpha = 0.85f
        wmgr.addView(iv, p)
        view = iv; wm = wmgr; params = p; visible = true
    }

    fun hide() {
        try { view?.let { wm?.removeView(it) } } catch (_: Exception) {}
        view = null; wm = null; params = null; visible = false
    }

    fun update(x: Float, y: Float) {
        val v = view ?: return
        val p = params ?: return
        p.x = x.toInt().coerceAtLeast(0)
        p.y = y.toInt().coerceAtLeast(0)
        v.post { try { wm?.updateViewLayout(v, p) } catch (_: Exception) {} }
    }
}
