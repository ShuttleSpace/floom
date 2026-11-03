package dev.floom.plugin.secure_storage;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;

import java.security.Key;
import java.security.KeyStore;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.spec.GCMParameterSpec;

public final class SecureStoragePlugin {
    private SecureStoragePlugin() {}

    private static final int MODE_PRIVATE = 0;
    private static final int PURPOSE_ENCRYPT_OR_DECRYPT = KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT;
    private static final int GCM_TAG_BITS = 128;

    private static String alias(String service) {
        return "floom-secure-" + service;
    }

    public static String get(Context context, String service, String key) {
        try {
            SharedPreferences prefs = context.getSharedPreferences("floom-secure-" + service, MODE_PRIVATE);
            String stored = prefs.getString(key, null);
            if (stored == null) {
                return null;
            }
            String[] parts = stored.split(":", 2);
            if (parts.length != 2) {
                return null;
            }
            byte[] iv = hexDecode(parts[0]);
            byte[] ciphertext = hexDecode(parts[1]);
            if (iv == null || ciphertext == null) {
                return null;
            }

            KeyStore store = KeyStore.getInstance("AndroidKeyStore");
            store.load(null);
            Key secret = store.getKey(alias(service), null);
            if (secret == null) {
                return null;
            }

            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE, secret, new GCMParameterSpec(GCM_TAG_BITS, iv));
            byte[] plaintext = cipher.doFinal(ciphertext);
            return new String(plaintext, java.nio.charset.StandardCharsets.UTF_8);
        } catch (Throwable unreachable) {
            return null;
        }
    }

    public static boolean set(Context context, String service, String key, String value) {
        try {
            KeyStore store = KeyStore.getInstance("AndroidKeyStore");
            store.load(null);
            Key secret;
            if (!store.containsAlias(alias(service))) {
                generateKey(store, alias(service));
            }
            secret = store.getKey(alias(service), null);
            if (secret == null) {
                return false;
            }

            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.ENCRYPT_MODE, secret);
            byte[] iv = cipher.getIV();
            byte[] plaintext = value.getBytes(java.nio.charset.StandardCharsets.UTF_8);
            byte[] ciphertext = cipher.doFinal(plaintext);

            SharedPreferences prefs = context.getSharedPreferences("floom-secure-" + service, MODE_PRIVATE);
            SharedPreferences.Editor editor = prefs.edit();
            editor.putString(key, hexEncode(iv) + ":" + hexEncode(ciphertext));
            return editor.commit();
        } catch (Throwable unreachable) {
            return false;
        }
    }

    public static boolean remove(Context context, String service, String key) {
        try {
            SharedPreferences prefs = context.getSharedPreferences("floom-secure-" + service, MODE_PRIVATE);
            SharedPreferences.Editor editor = prefs.edit();
            editor.remove(key);
            return editor.commit();
        } catch (Throwable unreachable) {
            return false;
        }
    }

    private static void generateKey(KeyStore store, String alias) throws Exception {
        KeyGenerator generator = KeyGenerator.getInstance("AES", "AndroidKeyStore");
        KeyGenParameterSpec.Builder specBuilder =
                new KeyGenParameterSpec.Builder(alias, PURPOSE_ENCRYPT_OR_DECRYPT);
        specBuilder.setBlockModes(new String[] {"GCM"});
        specBuilder.setEncryptionPaddings(new String[] {"NoPadding"});
        generator.init(specBuilder.build());
        generator.generateKey();
    }

    private static String hexEncode(byte[] bytes) {
        StringBuilder out = new StringBuilder(bytes.length * 2);
        for (byte b : bytes) {
            out.append(Character.forDigit((b >> 4) & 0xF, 16));
            out.append(Character.forDigit(b & 0xF, 16));
        }
        return out.toString();
    }

    private static byte[] hexDecode(String hex) {
        if (hex == null || (hex.length() & 1) != 0) {
            return null;
        }
        byte[] out = new byte[hex.length() / 2];
        for (int i = 0; i < hex.length(); i += 2) {
            int high = Character.digit(hex.charAt(i), 16);
            int low = Character.digit(hex.charAt(i + 1), 16);
            if (high < 0 || low < 0) {
                return null;
            }
            out[i / 2] = (byte) ((high << 4) | low);
        }
        return out;
    }
}
