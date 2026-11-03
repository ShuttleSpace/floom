//! Android encrypted storage via `SecureStoragePlugin`.
//!
//! The capability is Java: the Android Keystore, `Cipher` and
//! `SharedPreferences` are Android APIs, and `SecureStoragePlugin` owns them.
//! This file only carries the ask across JNI.

use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::JValue;
use jni::strings::JNIString;

/// `SecureStoragePlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.secure_storage.SecureStoragePlugin";

pub fn get(service: &str, key: &str) -> Result<Option<String>, String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let service = env.new_string(service).e()?;
        let key = env.new_string(key).e()?;
        let value = env
            .call_static_method(
                &class,
                jni::jni_str!("get"),
                jni::jni_sig!(
                    "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
                ),
                &[
                    JValue::Object(&context),
                    JValue::Object(&service),
                    JValue::Object(&key),
                ],
            )
            .e()?
            .l()
            .e()?;
        if value.is_null() {
            return Ok(None);
        }
        let string = env
            .as_cast::<jni::objects::JString<'_>>(&value)
            .map_err(|e| e.to_string())?;
        Ok(string.try_to_string(env).ok())
    })
}

pub fn set(service: &str, key: &str, value: &str) -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let service = env.new_string(service).e()?;
        let key = env.new_string(key).e()?;
        let value = env.new_string(value).e()?;
        let wrote = env
            .call_static_method(
                &class,
                jni::jni_str!("set"),
                jni::jni_sig!(
                    "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Z"
                ),
                &[
                    JValue::Object(&context),
                    JValue::Object(&service),
                    JValue::Object(&key),
                    JValue::Object(&value),
                ],
            )
            .e()?
            .z()
            .e()?;
        if wrote {
            Ok(())
        } else {
            Err("secure-storage: SecureStoragePlugin.set() reported the write failed".into())
        }
    })
}

pub fn remove(service: &str, key: &str) -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let service = env.new_string(service).e()?;
        let key = env.new_string(key).e()?;
        let wrote = env
            .call_static_method(
                &class,
                jni::jni_str!("remove"),
                jni::jni_sig!("(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;)Z"),
                &[
                    JValue::Object(&context),
                    JValue::Object(&service),
                    JValue::Object(&key),
                ],
            )
            .e()?
            .z()
            .e()?;
        if wrote {
            Ok(())
        } else {
            Err("secure-storage: SecureStoragePlugin.remove() reported the write failed".into())
        }
    })
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
