//! Android: an AES-256-GCM key held by the Android Keystore, ciphertext in
//! `SharedPreferences`.
//!
//! `EncryptedSharedPreferences` would do this for us, but it lives in
//! `androidx.security:security-crypto` — a Gradle dependency this side cannot
//! add. `java.security.KeyStore` and `javax.crypto` are framework classes, so
//! the JNI bridge reaches them without app-side Java.
//!
//! Layout: one Keystore alias per store (`floom-secure-<service>`), one
//! `SharedPreferences` file per store, one entry per key holding
//! `<hex iv>:<hex ciphertext>`. GCM's IV is generated per write and carried
//! with the ciphertext; nothing else about the value is stored.

use floom_platform_mobile::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::{JByteArray, JObject};

use crate::hex;

/// `Context.MODE_PRIVATE`.
const MODE_PRIVATE: i32 = 0;
/// `KeyProperties.PURPOSE_ENCRYPT | PURPOSE_DECRYPT`.
const PURPOSE_ENCRYPT_OR_DECRYPT: i32 = 3;
/// `Cipher.ENCRYPT_MODE`.
const ENCRYPT_MODE: i32 = 1;
/// `Cipher.DECRYPT_MODE`.
const DECRYPT_MODE: i32 = 2;
/// GCM tag length, in bits.
const GCM_TAG_BITS: i32 = 128;

pub fn get(service: &str, key: &str) -> Result<Option<String>, String> {
    jni_helpers::with_env(|env| {
        let Some(stored) = stored_value(env, service, key)? else {
            return Ok(None);
        };
        let (iv, ciphertext) = split_stored(&stored)?;

        let class = env
            .find_class(jni::jni_str!("javax/crypto/spec/GCMParameterSpec"))
            .e()?;
        let iv = env.byte_array_from_slice(&iv).e()?;
        let spec = env
            .new_object(
                &class,
                jni::jni_sig!("(I[B)V"),
                &[jni::objects::JValue::Int(GCM_TAG_BITS), obj(&iv)],
            )
            .e()?;

        let cipher = cipher(env)?;
        let secret = secret_key(env, service)?;
        env.call_method(
            &cipher,
            jni::jni_str!("init"),
            jni::jni_sig!("(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V"),
            &[
                jni::objects::JValue::Int(DECRYPT_MODE),
                obj(&secret),
                obj(&spec),
            ],
        )
        .e()?;

        let ciphertext = env.byte_array_from_slice(&ciphertext).e()?;
        let plaintext = env
            .call_method(
                &cipher,
                jni::jni_str!("doFinal"),
                jni::jni_sig!("([B)[B"),
                &[obj(&ciphertext)],
            )
            .and_then(|v| v.l())
            .e()?;
        let plaintext = env
            .as_cast::<JByteArray<'_>>(&plaintext)
            .map_err(|e| e.to_string())?;
        let bytes = env.convert_byte_array(&plaintext).e()?;
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| format!("secure-storage: the value under {key:?} is not UTF-8"))
    })
}

pub fn set(service: &str, key: &str, value: &str) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let cipher = cipher(env)?;
        let secret = secret_key(env, service)?;
        env.call_method(
            &cipher,
            jni::jni_str!("init"),
            jni::jni_sig!("(ILjava/security/Key;)V"),
            &[jni::objects::JValue::Int(ENCRYPT_MODE), obj(&secret)],
        )
        .e()?;

        let plaintext = env.byte_array_from_slice(value.as_bytes()).e()?;
        let iv = env
            .call_method(&cipher, jni::jni_str!("getIV"), jni::jni_sig!("()[B"), &[])
            .and_then(|v| v.l())
            .e()?;
        let iv = env
            .as_cast::<JByteArray<'_>>(&iv)
            .map_err(|e| e.to_string())?;
        let iv = env.convert_byte_array(&iv).e()?;

        let ciphertext = env
            .call_method(
                &cipher,
                jni::jni_str!("doFinal"),
                jni::jni_sig!("([B)[B"),
                &[obj(&plaintext)],
            )
            .and_then(|v| v.l())
            .e()?;
        let ciphertext = env
            .as_cast::<JByteArray<'_>>(&ciphertext)
            .map_err(|e| e.to_string())?;
        let ciphertext = env.convert_byte_array(&ciphertext).e()?;

        store_value(
            env,
            service,
            key,
            &format!("{}:{}", hex::encode(&iv), hex::encode(&ciphertext)),
        )
    })
}

pub fn remove(service: &str, key: &str) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let prefs = prefs(env, service)?;
        let editor = edit(env, &prefs)?;
        let key: JObject = env.new_string(key).e()?.into();
        env.call_method(
            &editor,
            jni::jni_str!("remove"),
            jni::jni_sig!("(Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;"),
            &[obj(&key)],
        )
        .e()?;
        commit(env, &editor)
    })
}

fn split_stored(stored: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    let (iv, ciphertext) = stored.split_once(':').ok_or_else(|| {
        "secure-storage: the stored value is not <iv>:<ciphertext>; it was not written by this crate"
            .to_string()
    })?;
    Ok((hex::decode(iv)?, hex::decode(ciphertext)?))
}

fn alias(service: &str) -> String {
    format!("floom-secure-{service}")
}

fn key_store<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let class = env
        .find_class(jni::jni_str!("java/security/KeyStore"))
        .e()?;
    let name: JObject = env.new_string("AndroidKeyStore").e()?.into();
    let store = env
        .call_static_method(
            &class,
            jni::jni_str!("getInstance"),
            jni::jni_sig!("(Ljava/lang/String;)Ljava/security/KeyStore;"),
            &[obj(&name)],
        )
        .and_then(|v| v.l())
        .e()?;
    if store.is_null() {
        return Err("secure-storage: KeyStore.getInstance returned null".into());
    }
    env.call_method(
        &store,
        jni::jni_str!("load"),
        jni::jni_sig!("(Ljava/security/KeyStore$LoadStoreParameter;)V"),
        &[obj(&JObject::null())],
    )
    .e()?;
    Ok(store)
}

/// The store's AES key, generated on first use. A missing key after the store
/// has been written means the Keystore entry was invalidated (a restore, a
/// lock-screen change) — an `Err`, not an empty store.
fn secret_key<'local>(
    env: &mut jni::Env<'local>,
    service: &str,
) -> Result<JObject<'local>, String> {
    let store = key_store(env)?;
    let name: JObject = env.new_string(alias(service)).e()?.into();
    let exists = env
        .call_method(
            &store,
            jni::jni_str!("containsAlias"),
            jni::jni_sig!("(Ljava/lang/String;)Z"),
            &[obj(&name)],
        )
        .and_then(|v| v.z())
        .e()?;
    if !exists {
        generate_key(env, &alias(service))?;
    }
    let key = env
        .call_method(
            &store,
            jni::jni_str!("getKey"),
            jni::jni_sig!("(Ljava/lang/String;[C)Ljava/security/Key;"),
            &[obj(&name), obj(&JObject::null())],
        )
        .and_then(|v| v.l())
        .e()?;
    if key.is_null() {
        return Err(format!(
            "secure-storage: the Android Keystore has no key for {service:?}"
        ));
    }
    Ok(key)
}

fn generate_key(env: &mut jni::Env<'_>, alias: &str) -> Result<(), String> {
    let class = env
        .find_class(jni::jni_str!(
            "android/security/keystore/KeyGenParameterSpec$Builder"
        ))
        .e()?;
    let name: JObject = env.new_string(alias).e()?.into();
    let builder = env
        .new_object(
            &class,
            jni::jni_sig!("(Ljava/lang/String;I)V"),
            &[
                obj(&name),
                jni::objects::JValue::Int(PURPOSE_ENCRYPT_OR_DECRYPT),
            ],
        )
        .e()?;
    let block_mode: JObject = env.new_string("GCM").e()?.into();
    let modes = env
        .new_object_array(1, jni::jni_str!("java/lang/String"), &block_mode)
        .e()?;
    let modes: JObject = modes.into();
    let builder = env
        .call_method(
            &builder,
            jni::jni_str!("setBlockModes"),
            jni::jni_sig!(
                "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;"
            ),
            &[obj(&modes)],
        )
        .and_then(|v| v.l())
        .e()?;
    let padding: JObject = env.new_string("NoPadding").e()?.into();
    let paddings = env
        .new_object_array(1, jni::jni_str!("java/lang/String"), &padding)
        .e()?;
    let paddings: JObject = paddings.into();
    let builder = env
        .call_method(
            &builder,
            jni::jni_str!("setEncryptionPaddings"),
            jni::jni_sig!(
                "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;"
            ),
            &[obj(&paddings)],
        )
        .and_then(|v| v.l())
        .e()?;
    let spec = env
        .call_method(
            &builder,
            jni::jni_str!("build"),
            jni::jni_sig!("()Landroid/security/keystore/KeyGenParameterSpec;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;

    let class = env
        .find_class(jni::jni_str!("javax/crypto/KeyGenerator"))
        .e()?;
    let algorithm: JObject = env.new_string("AES").e()?.into();
    let provider: JObject = env.new_string("AndroidKeyStore").e()?.into();
    let generator = env
        .call_static_method(
            &class,
            jni::jni_str!("getInstance"),
            jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;"),
            &[obj(&algorithm), obj(&provider)],
        )
        .and_then(|v| v.l())
        .e()?;
    env.call_method(
        &generator,
        jni::jni_str!("init"),
        jni::jni_sig!("(Ljava/security/spec/AlgorithmParameterSpec;)V"),
        &[obj(&spec)],
    )
    .e()?;
    env.call_method(
        &generator,
        jni::jni_str!("generateKey"),
        jni::jni_sig!("()Ljavax/crypto/SecretKey;"),
        &[],
    )
    .e()?;
    Ok(())
}

fn cipher<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let class = env.find_class(jni::jni_str!("javax/crypto/Cipher")).e()?;
    let transformation: JObject = env.new_string("AES/GCM/NoPadding").e()?.into();
    let cipher = env
        .call_static_method(
            &class,
            jni::jni_str!("getInstance"),
            jni::jni_sig!("(Ljava/lang/String;)Ljavax/crypto/Cipher;"),
            &[obj(&transformation)],
        )
        .and_then(|v| v.l())
        .e()?;
    if cipher.is_null() {
        return Err("secure-storage: Cipher.getInstance returned null".into());
    }
    Ok(cipher)
}

fn stored_value(
    env: &mut jni::Env<'_>,
    service: &str,
    key: &str,
) -> Result<Option<String>, String> {
    let prefs = prefs(env, service)?;
    let key: JObject = env.new_string(key).e()?.into();
    let value = env
        .call_method(
            &prefs,
            jni::jni_str!("getString"),
            jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"),
            &[obj(&key), obj(&JObject::null())],
        )
        .and_then(|v| v.l())
        .e()?;
    if value.is_null() {
        return Ok(None);
    }
    let value = env
        .as_cast::<jni::objects::JString<'_>>(&value)
        .map_err(|e| e.to_string())?;
    value
        .try_to_string(env)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn store_value(
    env: &mut jni::Env<'_>,
    service: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let prefs = prefs(env, service)?;
    let editor = edit(env, &prefs)?;
    let key: JObject = env.new_string(key).e()?.into();
    let value: JObject = env.new_string(value).e()?.into();
    env.call_method(
        &editor,
        jni::jni_str!("putString"),
        jni::jni_sig!(
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;"
        ),
        &[obj(&key), obj(&value)],
    )
    .e()?;
    commit(env, &editor)
}

fn prefs<'local>(env: &mut jni::Env<'local>, service: &str) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let name: JObject = env
        .new_string(&format!("floom-secure-{service}"))
        .e()?
        .into();
    let prefs = env
        .call_method(
            &activity,
            jni::jni_str!("getSharedPreferences"),
            jni::jni_sig!("(Ljava/lang/String;I)Landroid/content/SharedPreferences;"),
            &[obj(&name), jni::objects::JValue::Int(MODE_PRIVATE)],
        )
        .and_then(|v| v.l())
        .e()?;
    if prefs.is_null() {
        return Err("secure-storage: getSharedPreferences returned null".into());
    }
    Ok(prefs)
}

fn edit<'local>(
    env: &mut jni::Env<'local>,
    prefs: &JObject<'local>,
) -> Result<JObject<'local>, String> {
    let editor = env
        .call_method(
            prefs,
            jni::jni_str!("edit"),
            jni::jni_sig!("()Landroid/content/SharedPreferences$Editor;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    if editor.is_null() {
        return Err("secure-storage: SharedPreferences.edit() returned null".into());
    }
    Ok(editor)
}

fn commit(env: &mut jni::Env<'_>, editor: &JObject<'_>) -> Result<(), String> {
    let wrote = env
        .call_method(editor, jni::jni_str!("commit"), jni::jni_sig!("()Z"), &[])
        .and_then(|v| v.z())
        .e()?;
    if wrote {
        Ok(())
    } else {
        Err("secure-storage: SharedPreferences.commit() reported the write failed".into())
    }
}
