package com.crosslink.app.net

import android.os.Build
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

object UpdateChecker {
    // Manifest hosted in GitHub repo (raw). APK URL should point to a public GitHub Release asset.
    // NOTE: If the repo is private, raw manifest and release asset downloads require authentication;
    // for in-app OTA without auth, host update.json and the APK on a public URL (e.g. public repo/release).
    private const val MANIFEST_URL = "https://raw.githubusercontent.com/MarklanHampac/CrossLink/master/update.json"

    data class Result(val message: String, val downloadUrl: String? = null)

    fun check(currentVersionCode: Int = BuildConfigSafe.versionCode(), currentVersionName: String = BuildConfigSafe.versionName()): Result {
        if (MANIFEST_URL.isBlank() || MANIFEST_URL.contains("your-org/crosslink")) {
            return Result("Update check not configured. Set MANIFEST_URL in com.crosslink.app.net.UpdateChecker to a hosted update.json URL (HTTPS).")
        }
        return try {
            val conn = (URL(MANIFEST_URL).openConnection() as HttpURLConnection).apply {
                connectTimeout = 5000; readTimeout = 5000; requestMethod = "GET"
                setRequestProperty("User-Agent", "CrossLink-UpdateChecker")
            }
            val code = conn.responseCode
            val body = (if (code in 200..299) conn.inputStream else conn.errorStream)?.bufferedReader()?.readText().orEmpty()
            conn.disconnect()
            if (code !in 200..299) {
                return Result("Update check failed (HTTP $code). For private GitHub repos raw/asset URLs need auth and return 404/403 in-app. Host update.json and APK on a public HTTPS URL (or make repo/release public) and update MANIFEST_URL.")
            }
            val j = JSONObject(body)
            val latestCode = j.optInt("versionCode", -1)
            val latestName = j.optString("version", currentVersionName)
            val url = j.optString("url", "")
            val notes = j.optString("notes", "")
            if (latestCode > currentVersionCode || (latestCode == -1 && latestName != currentVersionName)) {
                Result("Update available: $latestName (current $currentVersionName). ${if (notes.isNotBlank()) notes else ""}".trim(), url.ifBlank { null })
            } else {
                Result("App is up to date ($currentVersionName).")
            }
        } catch (e: Exception) {
            Result("Update check unavailable: ${e.message}. Check internet access and MANIFEST_URL configuration.")
        }
    }
}

object BuildConfigSafe {
    fun versionCode(): Int = try {
        val c = Class.forName("com.crosslink.app.BuildConfig")
        c.getField("VERSION_CODE").getInt(null)
    } catch (_: Exception) { 1 }
    fun versionName(): String = try {
        val c = Class.forName("com.crosslink.app.BuildConfig")
        c.getField("VERSION_NAME").get(null) as? String ?: "0.1.0"
    } catch (_: Exception) { "0.1.0" }
}
