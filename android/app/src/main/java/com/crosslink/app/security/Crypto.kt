package com.crosslink.app.security

import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

object Crypto {
    fun randomPub(): ByteArray {
        val b = ByteArray(32)
        SecureRandom().nextBytes(b)
        return b
    }

    fun deriveKey(myPub: ByteArray, peerPub: ByteArray): SecretKey {
        val a = myPub; val b = peerPub
        val combined = if (compare(a,b) <= 0) a + b else b + a
        val digest = MessageDigest.getInstance("SHA-256").digest(combined)
        return SecretKeySpec(digest, "AES")
    }

    private fun compare(a: ByteArray, b: ByteArray): Int {
        val n = minOf(a.size, b.size)
        for (i in 0 until n) {
            val ai = a[i].toInt() and 0xFF
            val bi = b[i].toInt() and 0xFF
            if (ai != bi) return ai - bi
        }
        return a.size - b.size
    }

    fun encrypt(key: SecretKey, plain: ByteArray): ByteArray {
        val nonce = ByteArray(12)
        SecureRandom().nextBytes(nonce)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key, GCMParameterSpec(128, nonce))
        val ct = cipher.doFinal(plain)
        return nonce + ct
    }

    fun decrypt(key: SecretKey, frame: ByteArray): ByteArray {
        require(frame.size >= 28) { "frame too short" }
        val nonce = frame.copyOfRange(0, 12)
        val ct = frame.copyOfRange(12, frame.size)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, nonce))
        return cipher.doFinal(ct)
    }
}
