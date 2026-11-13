package com.crosslink.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.text.format.Formatter
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.crosslink.app.net.DiscoveryResponder
import com.crosslink.app.net.InputReceiverServer
import com.crosslink.app.net.UpdateChecker
import com.crosslink.app.service.CrossLinkAccessibilityService
import com.crosslink.app.ui.CursorOverlay
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : ComponentActivity() {
    private var server: InputReceiverServer? = null
    private var discovery: DiscoveryResponder? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val statusState = mutableStateOf("Starting server...")
        server = InputReceiverServer(8888) { s -> runOnUiThread { statusState.value = s } }
        server?.start()
        discovery = DiscoveryResponder(applicationContext, 8889).also { it.start() }
        if (discovery?.lastError != null) statusState.value = "Discovery error: ${discovery?.lastError}"

        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    MainDashboard(
                        localIp = getLocalIpAddress(),
                        status = statusState.value,
                        onOpenAccessibility = { startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)) },
                        onOpenIme = { startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)) },
                        onOpenOverlay = {
                            val intent = Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:$packageName"))
                            startActivity(intent)
                        },
                        onShowOverlay = { CursorOverlay.show(applicationContext) },
                        onHideOverlay = { CursorOverlay.hide() }
                    )
                }
            }
        }
    }

    private fun getLocalIpAddress(): String {
        return try {
            val wifiManager = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
            val ipAddress = wifiManager.connectionInfo?.ipAddress ?: 0
            if (ipAddress != 0) Formatter.formatIpAddress(ipAddress) else "0.0.0.0"
        } catch (_: Exception) { "0.0.0.0" }
    }

    override fun onDestroy() {
        discovery?.stop()
        server?.stop()
        CursorOverlay.hide()
        super.onDestroy()
    }
}

@Composable
fun MainDashboard(
    localIp: String,
    status: String,
    onOpenAccessibility: () -> Unit,
    onOpenIme: () -> Unit,
    onOpenOverlay: () -> Unit,
    onShowOverlay: () -> Unit,
    onHideOverlay: () -> Unit
) {
    val ctx = LocalContext.current
    val scope = rememberCoroutineScope()
    var updateMsg by remember { mutableStateOf<String?>(null) }
    var updateUrl by remember { mutableStateOf<String?>(null) }
    var checking by remember { mutableStateOf(false) }
    val a11yOn = CrossLinkAccessibilityService.instance != null

    Column(
        modifier = Modifier.fillMaxSize().padding(20.dp).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("CrossLink Companion", style = MaterialTheme.typography.headlineMedium)
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Phone IP: $localIp", style = MaterialTheme.typography.titleMedium)
                Text("Port: 8888 (TCP)  •  Discovery: 8889 (UDP)", style = MaterialTheme.typography.bodySmall)
                Text("Status: $status", style = MaterialTheme.typography.bodyMedium)
                Text(
                    "Accessibility: ${if (a11yOn) "ENABLED" else "DISABLED"}",
                    color = if (a11yOn) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error
                )
                Text("Make sure phone and PC are on the same Wi-Fi/network and firewall allows port 8888.", style = MaterialTheme.typography.bodySmall)
            }
        }

        Button(onClick = onOpenAccessibility, modifier = Modifier.fillMaxWidth()) { Text("Enable Accessibility Service") }
        Button(onClick = onOpenIme, modifier = Modifier.fillMaxWidth()) { Text("Select CrossLink Keyboard") }
        Button(onClick = onOpenOverlay, modifier = Modifier.fillMaxWidth()) { Text("Grant Overlay Permission") }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = onShowOverlay, modifier = Modifier.weight(1f)) { Text("Show Cursor") }
            OutlinedButton(onClick = onHideOverlay, modifier = Modifier.weight(1f)) { Text("Hide Cursor") }
        }

        Divider()
        Text("In-app update (OTA)", style = MaterialTheme.typography.titleMedium)
        Button(onClick = {
            checking = true; updateMsg = null; updateUrl = null
            scope.launch {
                val res = withContext(Dispatchers.IO) { UpdateChecker.check() }
                checking = false
                updateMsg = res.message
                updateUrl = res.downloadUrl
            }
        }, enabled = !checking, modifier = Modifier.fillMaxWidth()) {
            if (checking) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                    Text("Checking for updates...")
                }
            } else {
                Text("Check for updates")
            }
        }
        if (updateMsg != null) {
            Text(updateMsg!!, style = MaterialTheme.typography.bodySmall)
        }
        if (updateUrl != null) {
            Button(onClick = {
                val i = Intent(Intent.ACTION_VIEW, Uri.parse(updateUrl))
                ctx.startActivity(i)
            }, modifier = Modifier.fillMaxWidth()) { Text("Download / Install update") }
        }
        Text("To enable OTA: host update.json (version, versionCode, url, notes) over HTTPS and set MANIFEST_URL in UpdateChecker.kt, then rebuild APK.", style = MaterialTheme.typography.bodySmall)
    }
}
