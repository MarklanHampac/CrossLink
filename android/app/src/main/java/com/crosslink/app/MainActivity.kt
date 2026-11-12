package com.crosslink.app

import android.content.Intent
import android.net.wifi.WifiManager
import android.os.Bundle
import android.provider.Settings
import android.text.format.Formatter
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.crosslink.app.net.InputReceiverServer
import com.crosslink.app.service.CrossLinkAccessibilityService

class MainActivity : ComponentActivity() {
    private var server: InputReceiverServer? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate()

        server = InputReceiverServer(8888)
        server?.start()

        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    MainDashboard(
                        localIp = getLocalIpAddress(),
                        onOpenAccessibilitySettings = {
                            startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
                        },
                        onOpenImeSettings = {
                            startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS))
                        }
                    )
                }
            }
        }
    }

    private fun getLocalIpAddress(): String {
        val wifiManager = applicationContext.getSystemService(WIFI_SERVICE) as? WifiManager
        val ipAddress = wifiManager?.connectionInfo?.ipAddress ?: 0
        return if (ipAddress != 0) {
            Formatter.formatIpAddress(ipAddress)
        } else {
            "127.0.0.1"
        }
    }

    override fun onDestroy() {
        server?.stop()
        super.onDestroy()
    }
}

@Composable
fun MainDashboard(
    localIp: String,
    onOpenAccessibilitySettings: () -> Unit,
    onOpenImeSettings: () -> Unit
) {
    val isAccessibilityEnabled = CrossLinkAccessibilityService.instance != null

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "CrossLink Companion",
            style = MaterialTheme.typography.headlineMedium
        )

        Card(
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(text = "Device IP: $localIp", style = MaterialTheme.typography.titleMedium)
                Text(text = "Port: 8888", style = MaterialTheme.typography.bodyMedium)
                Text(
                    text = if (isAccessibilityEnabled) "Accessibility Service: ENABLED" else "Accessibility Service: DISABLED",
                    color = if (isAccessibilityEnabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall
                )
            }
        }

        Button(
            onClick = onOpenAccessibilitySettings,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Enable Accessibility Service")
        }

        Button(
            onClick = onOpenImeSettings,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Select CrossLink Keyboard")
        }
    }
}