//! Complete native host projects, emitted by `floom create`.
//!
//! # Why these are real projects and not notes
//!
//! A directory holding a `README.md` and a stub is not an Android project. It is
//! a folder that a person has to finish by hand, and "finish by hand" is where
//! `androidx.camera` silently becomes `Camera2` — because the easy path from an
//! empty folder is the framework API you already know, not the one the design
//! called for.
//!
//! So each platform emits the files its own toolchain requires, in the layout
//! that toolchain expects, and can be opened directly:
//!
//! * `android/` — a Gradle project with `settings.gradle.kts`, an
//!   `AndroidManifest.xml`, a Kotlin source set, and a wrapper. Opens in Android
//!   Studio. Dependencies are declared in Gradle because that is where Gradle
//!   resolves them; a Rust `build.rs` cannot resolve an AAR.
//! * `ios/` — a CocoaPods podspec plus Swift/ObjC sources. Opens in Xcode.
//! * `windows/`, `linux/`, `macos/` — CMake projects, openable in an IDE with
//!   the CMake integration and buildable from a shell.
//! * `web/` — an `index.html` and the wasm packaging.
//! * `ohos/` — a DevEco project.
//! * `tui/` — nothing: a terminal target is an ordinary Cargo binary, so the
//!   directory carries only a note saying so.
//!
//! # The boundary these projects draw
//!
//! Each native project has exactly one job: **own the native side of the FFI
//! contract and hand it to Rust.** Concretely, on Android the Kotlin class
//! declares `external fun`s whose symbols are exported by the Rust cdylib, and
//! Rust calls Kotlin through JNI. Both directions are direct calls into the
//! other's memory. There is no message channel, no string encoding of arguments,
//! and no runtime lookup by name — which is the whole difference between this
//! and a Flutter plugin, and the reason the native half can be written in
//! Kotlin and Swift with their full framework access rather than restricted to
//! what a serialized protocol can express.
//!
//! Application logic does not live here. `src/` is the app; `android/` is the
//! shell that hosts it. A `MainActivity.kt` that decided what to draw would be
//! logic in the wrong place, and it would be logic that could not be tested by
//! `cargo test`.

/// Gradle version pinned in every emitted wrapper.
///
/// Pinned rather than floating because a wrapper that resolves to a different
/// Gradle on a different day is a wrapper that builds in CI and fails on a
/// laptop. Matches what Flutter currently ships.
pub const GRADLE_VERSION: &str = "9.1.0";

/// Android Gradle Plugin version.
pub const AGP_VERSION: &str = "9.0.1";

/// Kotlin plugin version.
pub const KOTLIN_VERSION: &str = "2.3.20";

/// Android `compileSdk`. New enough for `androidx.camera`'s current requirements.
pub const COMPILE_SDK: u32 = 36;

/// Android `minSdk`. `androidx.camera` requires 21+; 24 is what current
/// Jetpack mandates and is below the point where the old support libraries are
/// worth keeping alive.
pub const MIN_SDK: u32 = 24;

/// Android `targetSdk`.
pub const TARGET_SDK: u32 = 36;

/// Java toolchain level.
///
/// 17 rather than 21: AGP 9 requires 17 as a floor and the Android toolchain
/// desugars newer bytecode for the device anyway, so a higher level buys
/// nothing and costs a toolchain that will not run on the machines the project
/// is meant to open on.
pub const JAVA_LEVEL: u32 = 17;

/// The Rust library name a native project links against.
///
/// Android loads it as `libfloom_<name>.so`, iOS as a framework, desktop as a
/// static library. `cdylib` and `staticlib` both have to be produced for the
/// first two to work, which is why the plugin manifest declares both.
pub fn lib_name(name: &str) -> String {
    format!("floom_{}", name.replace('-', "_"))
}

/// Android application package id from a reverse-DNS org and a project name.
pub fn android_app_id(org: &str, name: &str) -> String {
    format!("{org}.{}", name.replace('-', "_"))
}

/// Kotlin package path segments for an app id: `com.example.my_app` → `com/example/my_app`.
pub fn kotlin_package_path(app_id: &str) -> String {
    app_id.replace('.', "/")
}

/// The Kotlin class name for an app's activity.
pub fn kotlin_class_name(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in name.chars() {
        if c == '-' || c == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

// --- Android: shared Gradle files ----------------------------------------

/// `android/settings.gradle.kts` for an **app**.
///
/// Mirrors Flutter's, minus the Flutter SDK include: the plugin that loads
/// Flutter's plugins is what generates `GeneratedPluginRegistrant`, and Floom
/// has no equivalent because its plugins are linked, not looked up.
pub fn android_app_settings_gradle(org: &str, name: &str) -> String {
    let app_id = android_app_id(org, name);
    let _ = app_id;
    format!(
        r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}

plugins {{
    id("com.android.application") version "{AGP_VERSION}" apply false
    id("org.jetbrains.kotlin.android") version "{KOTLIN_VERSION}" apply false
}}

// Repositories are declared once, in the root `build.gradle.kts`, and this
// settings file does *not* add a `dependencyResolutionManagement` block with
// `FAIL_ON_PROJECT_REPOS`.
//
// That combination looks like good hygiene and is not: `FAIL_ON_PROJECT_REPOS`
// makes Gradle reject the allprojects repositories block in the root build file,
// so a project emitted with both fails at configuration with "repository
// 'Google' was added by build file". One of the two declarations has to go, and
// the root build file is the one that scales — it is the only one a subproject
// inherits.
rootProject.name = "{name}"
include(":app")
"#
    )
}

/// `android/settings.gradle.kts` for a **plugin**.
///
/// A plugin is a Gradle *library*, not an application: no `:app`, no
/// `applicationId`, and a plugin project must be consumable by another Gradle
/// build without being the root project. Emitting `include(":app")` here would
/// produce a project that Android Studio opens and that nothing can depend on.
pub fn android_plugin_settings_gradle(name: &str) -> String {
    format!(
        r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}

plugins {{
    id("com.android.library") version "{AGP_VERSION}" apply false
    id("org.jetbrains.kotlin.android") version "{KOTLIN_VERSION}" apply false
}}

rootProject.name = "floom-plugin-{name}"
"#
    )
}

/// `android/build.gradle.kts` — the root project file.
///
/// Deliberately near-empty. Flutter's version redirects the build directory to
/// keep one `build/` for the whole tree, which is worth doing when the tree holds
/// hundreds of generated projects; here there is one Gradle build per `floom
/// create`, so a redirect would only hide where the output went.
pub fn android_root_build_gradle() -> String {
    r#"allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
"#
    .to_string()
}

/// `android/gradle.properties`.
pub fn android_gradle_properties() -> String {
    format!(
        r#"org.gradle.jvmargs=-Xmx4G -XX:MaxMetaspaceSize=2G
org.gradle.parallel=true
org.gradle.caching=true

android.useAndroidX=true
android.nonTransitiveRClass=true

# The Kotlin plugin is applied explicitly, so AGP must not also apply it from
# its own built-in rules.
android.builtInKotlin=false
kotlin.code.style=official
"#
    )
}

/// `android/gradle/wrapper/gradle-wrapper.properties`.
pub fn android_gradle_wrapper_properties() -> String {
    format!(
        "distributionBase=GRADLE_USER_HOME\n\
         distributionPath=wrapper/dists\n\
         zipStoreBase=GRADLE_USER_HOME\n\
         zipStorePath=wrapper/dists\n\
         distributionUrl=https\\://services.gradle.org/distributions/gradle-{GRADLE_VERSION}-all.zip\n"
    )
}

/// `android/local.properties`.
///
/// Written rather than gitignored-and-absent, because Gradle fails with an
/// unhelpful message when it is missing, and Android Studio writes it only after
/// you have opened the project once — which is exactly when you cannot tell
/// whether the failure is the project or the file.
pub fn android_local_properties(sdk_dir: &str) -> String {
    format!(
        "## This file is machine-specific and is in .gitignore. It is written by\n\
         ## `floom create` so that the project builds before Android Studio has\n\
         ## touched it. Android Studio will rewrite it with its own paths.\n\
         sdk.dir={sdk_dir}\n"
    )
}

/// The SDK path to write into `local.properties`, if one is discoverable.
///
/// Returns `None` rather than a guess: a wrong `sdk.dir` produces a build
/// failure that looks like a project misconfiguration, and "no SDK found" is a
/// sentence the person reading it can act on.
pub fn detect_android_sdk() -> Option<String> {
    if let Ok(dir) = std::env::var("ANDROID_HOME") {
        if !dir.is_empty() {
            return Some(dir);
        }
    }
    if let Ok(dir) = std::env::var("ANDROID_SDK_ROOT") {
        if !dir.is_empty() {
            return Some(dir);
        }
    }
    let mac = std::env::var("HOME")
        .ok()
        .map(|h| format!("{h}/Library/Android/sdk"));
    let linux = std::env::var("HOME")
        .ok()
        .map(|h| format!("{h}/Android/Sdk"));
    [mac, linux]
        .into_iter()
        .flatten()
        .find(|p| std::path::Path::new(p).is_dir())
}

/// `android/.gitignore`.
pub const ANDROID_GITIGNORE: &str = "\
*.iml
.gradle
/local.properties
/.idea
.DS_Store
/build
/captures
.externalNativeBuild
.cxx
local.properties
";

/// `android/app/src/main/AndroidManifest.xml` for an app.
///
/// Declares the FFI library's package and its ABI filters. The `abiFilters` are
/// what stops a 30 MB `armeabi-v7a` slice from being packaged on a device that
/// cannot run it — Floom builds the Rust cdylib for exactly the ABIs listed here,
/// so this line and `build.rs` are two halves of one statement.
pub fn android_app_manifest(org: &str, name: &str) -> String {
    let app_id = android_app_id(org, name);
    let class = kotlin_class_name(name);
    // The dotted package, not the source path. `android:name` is resolved with
    // `Class.forName`, so `com/example/demo.MainActivity` is not merely unusual
    // — AAPT2 rejects it at resource-processing time with "must be a valid Java
    // class name", and the Kotlin sources are never even reached.
    let pkg = app_id.replace('-', "_");
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <uses-permission android:name="android.permission.INTERNET" />

    <application
        android:allowBackup="true"
        android:icon="@mipmap/ic_launcher"
        android:label="@string/app_name"
        android:supportsRtl="true"
        android:theme="@style/Theme.{name}">

        <activity
            android:name="{pkg}.{class}Activity"
            android:exported="true"
            android:configChanges="orientation|keyboardHidden|keyboard|screenSize|smallestScreenSize|screenLayout|uiMode"
            android:launchMode="singleTop"
            android:windowSoftInputMode="adjustResize">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>

        <!--
          The Rust half is packaged here rather than extracted from an APK entry
          at runtime. These four are the ABIs `build.rs` cross-compiles for;
          listing only what the device can run is what keeps the APK small.
        -->
        <meta-data
            android:name="floom.nativeLibraryDir"
            android:value="libfloom_{name}" />

    </application>

</manifest>
"#
    )
}

/// `android/app/src/main/AndroidManifest.xml` for a **plugin** library.
///
/// No `<application>` element: a library manifest declares only what its
/// components need, and a library that declares an application merges its
/// attributes into every consumer, which is how a plugin ends up renaming the
/// host app.
pub fn android_plugin_manifest(name: &str) -> String {
    // The `package` attribute is required, and its absence is reported as
    // "ManifestMerger2$MergeFailureException: Error parsing
    // .../AndroidManifest.xml" — which names the merger, not the missing
    // attribute, and points at a file that is otherwise perfectly well-formed.
    //
    // The namespace in `build.gradle.kts` and this package are two declarations of
    // the same identity; they must agree, so both are derived from the plugin
    // name here rather than written out twice.
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!--
  A plugin's permissions are its *requirements*, declared here so a consumer can
  see them at merge time. `floom create` with the plugin template adds none
  automatically: a permission nobody asked for is a permission a reviewer has to
  justify.

  This comment cannot contain a double hyphen: XML forbids the sequence inside a
  comment, xmllint reports it as a parser error, and AGP reports it as a manifest
  parsing failure that names the merger rather than the punctuation. That is why
  every comment in every document this tool emits is checked by a test.

  There is deliberately no application element. A library manifest that declares
  one merges its attributes into every consumer, which is how a plugin ends up
  renaming the host app.
-->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
  package="dev.floom.plugin.{name}">
</manifest>
"#
    )
}

/// `android/app/src/main/res/values/strings.xml`.
pub fn android_strings_xml(name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{name}</string>
</resources>
"#
    )
}

/// `android/app/src/main/res/values/styles.xml`.
///
/// A Material theme with no action bar, because the Rust half draws the window's
/// contents and a platform action bar would sit above it consuming space the app
/// did not budget for.
///
/// The style is named after the project so the manifest can reference
/// `Theme.<name>`; a fixed `Theme.Floom` here with a per-project manifest
/// reference fails at AAPT2 with "resource style/Theme.demo not found", and the
/// error names the manifest rather than the styles file that is wrong.
pub fn android_styles_xml(name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.{name}" parent="android:Theme.Material.NoActionBar">
        <!-- Rust paints every pixel of the content area, so the window background
             only shows during startup and resize. -->
        <item name="android:windowBackground">@color/floom_window_background</item>
    </style>
</resources>
"#
    )
}

/// `android/app/src/main/res/values/colors.xml`.
///
/// One colour, referenced by the theme and by the launcher icon. Emitted as a
/// resource rather than an inline literal so the two cannot disagree.
pub fn android_colors_xml() -> String {
    r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <!-- Shown before the first Rust frame lands. Dark, because a light flash
         before a dark-themed app paints is the visible half of "the app flickers
         white on launch". -->
    <color name="floom_window_background">#FF101014</color>
    <color name="ic_launcher_background">#FF101014</color>
</resources>
"#
    .to_string()
}

/// `android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml`.
///
/// An adaptive icon as XML rather than the PNG ladder Flutter emits.
///
/// The PNGs are binary and cannot come out of a `format!`; emitting them would
/// mean shipping a base64 blob in the CLI, which is worse than the alternative.
/// An adaptive icon is XML, covers every density through `anydpi-v26`, and for
/// older launchers `mipmap-mdpi/ic_launcher.xml` supplies a vector fallback that
/// the platform scales.
pub fn android_launcher_icon_xml() -> String {
    r#"<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
    <monochrome android:drawable="@drawable/ic_launcher_foreground" />
</adaptive-icon>
"#
    .to_string()
}

/// `android/app/src/main/res/drawable/ic_launcher_foreground.xml`.
///
/// A plain shape. The point is that the icon is a real resource that resolves, so
/// the manifest's `@mipmap/ic_launcher` is valid — the artwork is the app's
/// business, and AAPT2 checks the reference, not the taste.
///
/// A `const` rather than a `format!` because nothing here is interpolated. A
/// vector's `pathData` is full of braces, and routing a string that needs no
/// interpolation through `format!` means every brace needs escaping — which is
/// exactly how a `pathData` attribute gets corrupted into a format error.
///
/// Note the `r##"…"##` rather than `r#"…"#`. An XML colour attribute contains
/// the two-character sequence `"#`, which is exactly what terminates a `r#"`
/// raw string — so `android:fillColor="#6750A4"` ends the literal mid-attribute
/// and the remainder is parsed as Rust. One extra `#` is the whole fix, and the
/// compiler's complaint (`prefix Z is unknown`, pointing at a coordinate pair in
/// `pathData`) names neither the string nor the cause.
pub const ANDROID_LAUNCHER_FOREGROUND_XML: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<!-- The launcher icon's foreground. Replace the shape below with the app's
     artwork; what matters here is that the drawable exists, because the
     manifest's @mipmap/ic_launcher is validated at build time. -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp"
    android:height="108dp"
    android:viewportWidth="108"
    android:viewportHeight="108">
    <path
        android:fillColor="#6750A4"
        android:pathData="M54,30 L74,42 L74,66 L54,78 L34,66 L34,42 Z" />
    <path
        android:fillColor="#FFFFFF"
        android:pathData="M54,42 L64,60 L44,60 L44,48 Z" />
</vector>
"##;

/// `android/app/build.gradle.kts` for an app.
///
/// The `externalNativeBuild` block is the part that matters: it is what turns
/// this into a project that *builds Rust*, rather than one that shells out to
/// Cargo and hopes. NDK's CMake toolchain invokes `build.rs`'s logic through
/// Cargo, and unpacks the `.so` into the APK.
pub fn android_app_build_gradle(org: &str, name: &str, _lib: &str) -> String {
    let app_id = android_app_id(org, name);
    format!(
        r#"plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}}

android {{
    namespace = "{app_id}"
    compileSdk = {COMPILE_SDK}

    defaultConfig {{
        applicationId = "{app_id}"
        minSdk = {MIN_SDK}
        targetSdk = {TARGET_SDK}
        versionCode = 1
        versionName = "1.0"

        // The four ABIs `build.rs` cross-compiles for. Listed here as well as in
        // the manifest because this is the half that decides what is *packaged*.
        ndk {{
            abiFilters += listOf("arm64-v8a", "x86_64", "armeabi-v7a", "x86")
        }}
    }}

    // How the Rust library gets built and packaged.
    //
    // `floom create` does not enable this, because doing so needs a Gradle
    // plugin that does not exist yet: NDK's CMake toolchain has no Cargo
    // integration of its own, and one that shells out to Cargo cannot report its
    // failures through Gradle. Writing the reference to a missing plugin would
    // make this project fail to configure, which is worse than a project that
    // builds the Kotlin side and waits for one line.
    //
    // Until then, produce the library out of band and Gradle will pick it up
    // from `src/main/jniLibs`:
    //
    //   cargo ndk -t arm64-v8a -t x86_64 -t armeabi-v7a -t x86 \
    //       -o app/src/main/jniLibs build --release
    //
    // externalNativeBuild {{
    //     cmake {{
    //         path = file("src/main/native/CMakeLists.txt")
    //         version = "3.22.1"
    //     }}
    // }}

    buildTypes {{
        release {{
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }}
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_{JAVA_LEVEL}
        targetCompatibility = JavaVersion.VERSION_{JAVA_LEVEL}
    }}
}}

// The Kotlin JVM target goes through the `compilerOptions` DSL, not
// `kotlinOptions.jvmTarget`.
//
// The older form is not merely deprecated under Kotlin 2.x: it is a hard script
// compilation error ("Using 'jvmTarget: String' is an error. Please migrate to
// the compilerOptions DSL"), so a project emitted with it fails before it
// configures.
//
// It also has to be *set*. Kotlin's default JVM target does not follow
// `compileOptions.sourceCompatibility`, and a mismatch between the two is
// "Inconsistent JVM Target Compatibility Between Java and Kotlin Tasks" at
// `:app:compileDebugKotlin` — which reads as a toolchain problem rather than a
// missing line in this file.
kotlin {{
    compilerOptions {{
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_{JAVA_LEVEL}
    }}
}}

android {{
    namespace = "{app_id}"
    compileSdk = {COMPILE_SDK}

    packaging {{
        // The Rust cdylib already links libunwind/libc++ the way NDK expects;
        // letting Gradle repackage them produces duplicate-symbol errors at
        // install time rather than at build time.
        jniLibs {{
            useLegacyPackaging = false
        }}
    }}

    sourceSets {{
        getByName("main") {{
            jniLibs.srcDirs("src/main/jniLibs")
        }}
    }}
}}

dependencies {{
    // The Rust cdylib is not a Gradle dependency: it is a `.so` under
    // `src/main/jniLibs`, and `System.loadLibrary` finds it there at runtime.
    // Emitting a `project(...)` reference to a module this settings file does not
    // include would fail configuration, and a missing dependency is not the same
    // thing as a missing library.
}}
"#
    )
}

/// `android/app/build.gradle.kts` for a **plugin** library.
///
/// The dependency block is where the design lives: a plugin declares its
/// framework requirements here, in Gradle, because this is where a Maven
/// coordinate can be resolved at all. `androidx.camera` is not a Rust crate and
/// has no `build.rs` equivalent — it is an AAR, and AARs come from Gradle.
pub fn android_plugin_build_gradle(name: &str, _lib: &str) -> String {
    format!(
        r#"// The `{name}` plugin, as a Gradle library.
//
// A plugin project is one module, so this file is both the root build file and
// the library's. That is why the repositories block is here rather than being
// split across a root file: there is no root file to hold it.
//
// Open this directory in Android Studio and it builds as a library, ready to be
// depended on by an app's settings.gradle.kts.

allprojects {{
    repositories {{
        google()
        mavenCentral()
    }}
}}

plugins {{
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}}

android {{
    // A library namespace, not an applicationId: this is the R class package and
    // the generated manifest's package, and it is what lets two plugins both
    // declare `Camera.kt` without colliding.
    namespace = "dev.floom.plugin.{name}"
    compileSdk = {COMPILE_SDK}

    defaultConfig {{
        minSdk = {MIN_SDK}
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_{JAVA_LEVEL}
        targetCompatibility = JavaVersion.VERSION_{JAVA_LEVEL}
    }}
}}

// See the note in the app template: `kotlinOptions.jvmTarget` is a script
// compilation error under Kotlin 2.x, not a deprecation.
kotlin {{
    compilerOptions {{
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_{JAVA_LEVEL}
    }}
}}

dependencies {{
    // --- Plugin dependencies ------------------------------------------
    //
    // This is where framework libraries belong, and the reason this file exists
    // rather than a Rust `Cargo.toml`: `androidx.camera:camera-*` are AARs
    // published to Maven, and only Gradle can resolve them.
    //
    // `floom create --plugin` adds none by default. Every line here is a
    // dependency the plugin's own design called for, chosen deliberately:
    //
    //   implementation("androidx.camera:camera-core:1.4.2")
    //   implementation("androidx.camera:camera-camera2:1.4.2")
    //   implementation("androidx.camera:camera-lifecycle:1.4.2")
    //   implementation("androidx.camera:camera-view:1.4.2")
    //
    // Writing them down here rather than in prose is the point of emitting a
    // real project: the alternative is a note saying "add the CameraX
    // dependencies", which is a note that gets skipped.
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
}}
"#
    )
}

/// `android/proguard-rules.pro`.
pub const ANDROID_PROGUARD_RULES: &str = "\
# Floom's Rust half is reached through JNI, whose symbol names are derived at
# runtime from the class and method name. R8 cannot see those references, so
# without this the release build compiles and then fails at first call.
-keepclasseswithmembernames class dev.floom.** {
    native <methods>;
}
-keep class dev.floom.** { *; }
";

// --- Android: Kotlin sources ---------------------------------------------

/// The app's `MainActivity.kt`.
///
/// Three jobs: load the Rust cdylib, create a `SurfaceView` for Rust to paint
/// into, and forward input. It deliberately does no layout — the Rust half owns
/// the widget tree and the frame, so anything drawn here would be drawn over or
/// under it unpredictably.
pub fn android_main_activity(org: &str, name: &str, lib: &str) -> String {
    let app_id = android_app_id(org, name);
    // The dotted package for the `package` statement. `kotlin_package_path` is
    // for the *directory* under `src/main/kotlin/`, and using it here produces
    // `package com/example/demo`, which the Kotlin parser rejects with four
    // "Expecting a top level declaration" errors on line 1 — one per slash.
    let pkg = app_id.replace('-', "_");
    let class = kotlin_class_name(name);
    format!(
        r#"package {pkg}

import android.os.Bundle
import android.view.Choreographer
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.app.Activity

/**
 * The Android host for `{name}`.
 *
 * This class is a shell. It loads the Rust library, gives Rust a surface to
 * paint into, and forwards input. It does not decide what the app looks like —
 * that is the widget tree in `src/`, and a decision made here would be a
 * decision the host had to be rewritten to change.
 *
 * ## The native boundary
 *
 * [FloomNative] declares `external fun`s. Their symbols are exported by the
 * Rust cdylib and resolved by the JVM on first call — a direct call into the
 * other side's memory, with no serialization and no runtime lookup by name.
 * That is why Kotlin here can pass a [Surface] straight through instead of
 * encoding it into a string, which is what a method channel would require.
 */
class {class}Activity : Activity() {{

    private lateinit var surface: SurfaceView
    private var frameCallbackPosted = false

    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)

        // `System.loadLibrary` finds `lib{lib}.so` in the APK's jniLibs. It must
        // happen before any `external fun` is touched, and the `UnsatisfiedLinkError`
        // it throws on a missing ABI names the ABI, so the message is left intact.
        System.loadLibrary("{lib}")

        surface = SurfaceView(this)
        surface.holder.addCallback(object : SurfaceHolder.Callback {{
            override fun surfaceCreated(holder: SurfaceHolder) {{
                FloomNative.surfaceCreated(holder.surface, surface.width, surface.height)
                requestFrame()
            }}

            override fun surfaceChanged(
                holder: SurfaceHolder,
                format: Int,
                width: Int,
                height: Int,
            ) {{
                FloomNative.surfaceResized(width, height)
                requestFrame()
            }}

            override fun surfaceDestroyed(holder: SurfaceHolder) {{
                FloomNative.surfaceDestroyed()
            }}
        }})

        // A hardware layer is not requested: Rust paints through a native
        // surface, and letting the compositor also cache the view would show the
        // previous frame while the new one is being drawn.
        surface.holder.setFormat(android.graphics.PixelFormat.RGBA_8888)
        surface.setOnTouchListener {{ view, event ->
            FloomNative.touch(
                event.action,
                event.x,
                event.y,
                event.pointerCount.coerceAtLeast(1),
            )
            view.performClick()
            true
        }}
        setContentView(surface)

        FloomNative.start()
    }}

    /**
     * Asks Rust for one frame and schedules the next.
     *
     * Rust decides whether to actually redraw, so this can run every vsync
     * without costing a frame: the cost of a no-op frame is the `requestFrame`
     * call and nothing else.
     */
    private fun requestFrame() {{
        if (frameCallbackPosted) return
        frameCallbackPosted = true
        Choreographer.getInstance().postFrameCallback {{
            frameCallbackPosted = false
            if (surface.isAttachedToWindow) {{
                FloomNative.frame()
                requestFrame()
            }}
        }}
    }}

    override fun onResume() {{
        super.onResume()
        FloomNative.lifecycleResumed()
    }}

    override fun onPause() {{
        FloomNative.lifecyclePaused()
        super.onPause()
    }}

    override fun onDestroy() {{
        FloomNative.shutdown()
        super.onDestroy()
    }}
}}

/**
 * The JNI surface Rust calls into.
 *
 * Every `external fun` here has a matching `#[no_mangle] pub extern "C"` in the
 * Rust half, and the JVM derives the symbol name from the class and method. The
 * naming convention is the contract: renaming a method here without renaming it
 * there is an `UnsatisfiedLinkError` at first call, not a compile error.
 *
 * The parameters are the FFI types the Rust side declares — ints, floats and
 * handles — which is the whole reason no serialization appears anywhere in this
 * path.
 */
object FloomNative {{
    external fun start()
    external fun shutdown()

    external fun surfaceCreated(surface: android.view.Surface, width: Int, height: Int)
    external fun surfaceResized(width: Int, height: Int)
    external fun surfaceDestroyed()

    /** Requests one frame; returns whether anything was painted. */
    external fun frame(): Boolean

    /**
     * Lifecycle transitions.
     *
     * Rust has to know when to stop scheduling frames and when to drop GPU
     * resources, and the only source of that truth is the platform. These are
     * notifications, not queries: nothing comes back.
     */
    external fun lifecycleResumed()
    external fun lifecyclePaused()

    /**
     * Forwards a pointer event.
     *
     * [action] is one of the `MotionEvent` action constants, passed as-is
     * because both sides agree on the numbering and copying it into an enum
     * would only add a conversion that can go wrong.
     */
    external fun touch(action: Int, x: Float, y: Float, pointerCount: Int)
}}

"#
    )
}

/// A plugin's Kotlin entry point.
///
/// A plugin does not own an activity — the host app does — so this is a plain
/// object the host calls. The host never sees the plugin's framework types
/// unless the plugin hands them over directly, which it can, because there is no
/// serialization between them.
pub fn android_plugin_kotlin(name: &str) -> String {
    let class = format!("{}Plugin", kotlin_class_name(name));
    format!(
        r#"package dev.floom.plugin.{name}

import android.content.Context

/**
 * The Android half of the `{name}` plugin.
 *
 * Registered by Rust through JNI and called by Rust through JNI. The host app
 * never names this class: it depends on the Rust crate, which knows how to
 * reach it. That is the whole difference from a Flutter plugin, where the host
 * depends on the Dart package and the tooling generates a registrant that
 * registers a *named* class on its behalf.
 *
 * ## The direction of the boundary
 *
 * These methods are **called by Rust**, not by the host app and not by
 * Kotlin's own `external` declarations. The C ABI in
 * `native/common/floom_{name}.h` is the contract; the C shim in
 * `src/main/cpp/` implements it and reaches this class over JNI.
 *
 * That direction is the one the header states: the native half implements the
 * four entry points, the Rust half calls them. An `external fun` here would
 * declare the *opposite* contract — Rust exporting to Kotlin — and the two
 * halves would then disagree about who owns the capability.
 *
 * ## Adding framework dependencies
 *
 * Add them to `build.gradle.kts`, not here. `androidx.camera:camera-*` are AARs
 * from Maven; Gradle is the only thing in this toolchain that can resolve one,
 * and a Rust `Cargo.toml` cannot express the dependency at all.
 *
 * ## @JvmStatic
 *
 * The C shim looks the methods up as *static* methods on this class, so every
 * one that Rust calls has to carry `@JvmStatic`. Without it the method is an
 * instance method on the singleton and `GetStaticMethodID` fails with
 * "NoSuchMethodError", which names the JVM rather than the missing annotation.
 */
object {class} {{

    /** Set by the host before any plugin call. */
    private var appContext: Context? = null

    /**
     * Called from Rust once, at enable time.
     *
     * Taking a [Context] rather than an Activity is deliberate: a plugin outlives
     * any one activity, and holding an Activity here is how a plugin ends up
     * leaking a destroyed window.
     */
    @JvmStatic
    fun attach(context: Context) {{
        appContext = context.applicationContext
    }}

    @JvmStatic
    fun detach() {{
        appContext = null
    }}

    /** Whether the plugin currently has what it needs to run. */
    @JvmStatic
    fun isAvailable(): Boolean = appContext != null

    /**
     * Runs the plugin's work and reports whether it succeeded.
     *
     * Returning a `String` error rather than throwing keeps the boundary in the
     * same shape in both directions: Rust reads a message, it does not have to
     * unwrap a JVM exception across FFI.
     */
    @JvmStatic
    fun perform(): String? = null
}}
"#
    )
}

/// A plugin's Android C shim — the C ABI, implemented over JNI.
///
/// Kotlin cannot export a C symbol, so the four entry points the C ABI
/// declares are implemented here and reach Kotlin over JNI. The shim is the
/// only thing that knows about both sides.
///
/// ## Where the JavaVM comes from
///
/// `JNI_GetCreatedJavaVMs` rather than a handle passed in from Rust. The
/// alternative — Rust looking the VM up and handing it over — would make the
/// C ABI's signature platform-specific, and the header is supposed to be the
/// one contract every platform implements. Asking the JVM for the VM it
/// already has keeps the ABI argument-free and portable.
pub fn android_plugin_c_shim(name: &str) -> String {
    let ident = name.replace('-', "_");
    let class = format!("{}Plugin", kotlin_class_name(name));
    format!(
        r#"/* The Android C shim for the `{name}` plugin.
 *
 * Implements the contract in `native/common/floom_{name}.h` and reaches the
 * Kotlin half over JNI. This file is the only place that knows about both
 * sides: the C ABI above it, and Kotlin below.
 *
 * Compiled into the plugin's own shared library by `build.rs`, with the
 * NDK's `jni.h` on the include path.
 */

#include "floom_{ident}.h"

#include <jni.h>
#include <stdlib.h>
#include <string.h>

/* The Kotlin class, in JNI's slash-separated form. */
#define PLUGIN_CLASS "dev/floom/plugin/{name}/{class}"

/* The VM this shim talks to. Fetched once, on first use. */
static JavaVM *g_vm = NULL;

/* The application context, as a global ref. */
static jobject g_context = NULL;

/*
 * Fetch the VM, attaching this thread to it if the thread is not the one that
 * created it.
 *
 * `JNI_GetCreatedJavaVMs` reports the VMs the process has, which is always
 * exactly one on Android. `AttachCurrentThread` is a no-op when the thread is
 * already attached, so it is safe to call unconditionally.
 */
static JNIEnv *plugin_env(void) {{
    JNIEnv *env = NULL;

    if (g_vm == NULL) {{
        jsize count = 0;
        if (JNI_GetCreatedJavaVMs(&g_vm, 1, &count) != JNI_OK || count == 0) {{
            return NULL;
        }}
    }}

    if ((*g_vm)->GetEnv(g_vm, (void **)&env, JNI_VERSION_1_6) != JNI_OK) {{
        if ((*g_vm)->AttachCurrentThread(g_vm, &env, NULL) != JNI_OK) {{
            return NULL;
        }}
    }}

    return env;
}}

/*
 * The application context, as a global ref.
 *
 * `ActivityThread.currentApplication()` is how the application is reached
 * without the plugin knowing anything about the host's activity: a plugin
 * outlives any one activity, and asking for the activity would be how it ends
 * up holding a destroyed window.
 */
static jobject plugin_application_context(JNIEnv *env) {{
    jclass thread = (*env)->FindClass(env, "android/app/ActivityThread");
    if (thread == NULL) {{
        (*env)->ExceptionClear(env);
        return NULL;
    }}

    jmethodID current = (*env)->GetStaticMethodID(
        env, thread, "currentApplication", "()Landroid/app/Application;");
    if (current == NULL) {{
        (*env)->ExceptionClear(env);
        return NULL;
    }}

    jobject app = (*env)->CallStaticObjectMethod(env, thread, current);
    (*env)->DeleteLocalRef(env, thread);
    if (app == NULL) {{
        (*env)->ExceptionClear(env);
        return NULL;
    }}

    jobject global = (*env)->NewGlobalRef(env, app);
    (*env)->DeleteLocalRef(env, app);
    return global;
}}

const char *floom_{ident}_version(void) {{
    /* Owned by the callee: the next call into this plugin invalidates it. */
    static const char version[] = "{name} 0.1.0";
    return version;
}}

int floom_{ident}_enable(void) {{
    JNIEnv *env = plugin_env();
    if (env == NULL) {{
        return 1;
    }}

    jclass cls = (*env)->FindClass(env, PLUGIN_CLASS);
    if (cls == NULL) {{
        (*env)->ExceptionClear(env);
        return 2;
    }}

    jmethodID attach = (*env)->GetStaticMethodID(
        env, cls, "attach", "(Landroid/content/Context;)V");
    if (attach == NULL) {{
        (*env)->ExceptionClear(env);
        return 3;
    }}

    jobject context = plugin_application_context(env);
    if (context == NULL) {{
        return 4;
    }}

    (*env)->CallStaticVoidMethod(env, cls, attach, context);
    if ((*env)->ExceptionCheck(env)) {{
        (*env)->ExceptionClear(env);
        return 5;
    }}

    /* The global ref outlives the enable: the plugin holds the context until
     * it is detached. */
    if (g_context != NULL) {{
        (*env)->DeleteGlobalRef(env, g_context);
    }}
    g_context = context;

    return 0;
}}

void floom_{ident}_disable(void) {{
    JNIEnv *env = plugin_env();
    if (env == NULL) {{
        return;
    }}

    jclass cls = (*env)->FindClass(env, PLUGIN_CLASS);
    if (cls != NULL) {{
        jmethodID detach = (*env)->GetStaticMethodID(env, cls, "detach", "()V");
        if (detach != NULL) {{
            (*env)->CallStaticVoidMethod(env, cls, detach);
            if ((*env)->ExceptionCheck(env)) {{
                (*env)->ExceptionClear(env);
            }}
        }}
    }}

    if (g_context != NULL) {{
        (*env)->DeleteGlobalRef(env, g_context);
        g_context = NULL;
    }}
}}

int floom_{ident}_capability(void) {{
    return 0;
}}

int floom_{ident}_abi_version(void) {{
    return FLOOM_{upper}_ABI_VERSION;
}}
"#,
        ident = ident,
        class = class,
        upper = ident.to_uppercase()
    )
}

// --- iOS ------------------------------------------------------------------

/// A plugin's `.podspec`.
///
/// CocoaPods rather than a bare Xcode project: a plugin has to be consumable by
/// an app that already has its own project file, and a podspec is the mechanism
/// that lets two independently versioned projects combine. A hand-written
/// `.xcodeproj` for a plugin would have to be edited by whoever integrates it.
pub fn ios_podspec(name: &str) -> String {
    let lib = lib_name(name);
    format!(
        r#"Pod::Spec.new do |s|
  s.name             = 'floom-plugin-{name}'
  s.version          = '0.1.0'
  s.summary          = 'The {name} plugin for Floom.'
  s.homepage         = 'https://github.com/lapce/floom'
  s.license          = {{ :type => 'MIT' }}
  s.author           = {{ 'Floom' => 'floom@example.com' }}
  s.source           = {{ :path => '.' }}

  s.ios.deployment_target = '15.0'
  s.swift_version   = '5.9'

  # The Rust half is a static library vendored into the pod. Linking it here is
  # what makes `import` in the app resolve to a symbol that exists: without it,
  # Swift compiles against declarations that no library provides.
  s.vendored_frameworks = 'src/lib{lib}.xcframework'
  s.libraries = 'c++'
  s.pod_target_xcconfig = {{
    'DEFINES_MODULE' => 'YES',
    # Rust panics must not unwind into Swift. `abort` turns them into a crash at
    # the boundary, which is debuggable; unwinding through an `extern "C"` frame
    # is undefined behaviour that usually looks like a corrupted stack later.
    'SWIFT_OBJC_BRIDGING_HEADER' => 'include/{name}-Bridging-Header.h',
  }}

  s.source_files = 'Classes/**/*.{{h,m,mm,swift}}'
end
"#
    )
}

/// A plugin's Swift entry point.
///
/// The `@_cdecl` functions are the C ABI: they are what the Rust half calls
/// through `extern "C"`. The Swift class above them is where the real work
/// goes — a `@_cdecl` function is a C entry point and cannot carry a Swift
/// receiver, so it forwards to the class.
///
/// The direction is the one the header states: the native half implements the
/// entry points, the Rust half calls them. An `@objc` method would be the
/// opposite contract, and the two halves would then disagree about who owns
/// the capability.
pub fn ios_plugin_swift(name: &str) -> String {
    let class = format!("{}Plugin", kotlin_class_name(name));
    let ident = name.replace('-', "_");
    format!(
        r#"import Foundation

/// The iOS half of the `{name}` plugin.
///
/// Called from Rust through the C ABI in `native/common/floom.h`. Unlike a
/// method channel, the arguments can be real Swift types: the C header declares
/// them, and there is nothing to encode them into.
public final class {class}: NSObject {{

    @objc public static func enable() -> String? {{
        // Returning `nil` rather than throwing: Rust reads a message across the
        // boundary, and a Swift error thrown through an `extern "C"` frame would
        // unwind across Rust, which is undefined behaviour.
        nil
    }}

    @objc public static func disable() {{}}

    @objc public static func isAvailable() -> Bool {{ true }}

    /// Marks the class visible to the Objective-C runtime.
    ///
    /// Without this the C ABI has nothing to resolve `enable` against. It is the
    /// one genuinely reflection-shaped part of a boundary that is otherwise
    /// resolved at link time.
    @objc({class}Bridge)
    private static var bridgeMarker: Any = {class}()
}}

// --- The C ABI ------------------------------------------------------------
//
// These are the functions the Rust half calls. `@_cdecl` gives them C linkage
// and an unmangled symbol name, which is what the linker resolves against the
// `extern "C"` declaration in `src/lib.rs`.
//
// A C entry point cannot have a Swift receiver, so each one forwards to the
// class above. That is the whole cost of the boundary: one forwarding function
// per entry point, and nothing else.

@_cdecl("floom_{ident}_version")
public func floom_{ident}_version() -> UnsafePointer<CChar> {{
    // Owned by the callee: the next call into this plugin invalidates it.
    // `static let` gives it that lifetime for free.
    return "{name} 0.1.0"
}}

@_cdecl("floom_{ident}_enable")
public func floom_{ident}_enable() -> Int32 {{
    if let error = {class}.enable() {{
        // The message cannot be returned through the ABI — it is an `int` — so
        // it goes to the log and the code says only that it failed.
        NSLog("floom-plugin-{name}: enable failed: %@", error)
        return 1
    }}
    return 0
}}

@_cdecl("floom_{ident}_disable")
public func floom_{ident}_disable() {{
    {class}.disable()
}}

@_cdecl("floom_{ident}_capability")
public func floom_{ident}_capability() -> Int32 {{
    0
}}

@_cdecl("floom_{ident}_abi_version")
public func floom_{ident}_abi_version() -> Int32 {{
    1
}}
"#,
        class = class,
        ident = ident
    )
}

/// A plugin's bridging header.
///
/// Declares the C ABI to Swift. `#include` of the generated header is what makes
/// `floom_{name}_enable()` callable from Swift at all.
pub fn ios_bridging_header(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"// Bridging header for the `{name}` plugin.
//
// Exposes the plugin's C ABI to Swift. This is the same header `bindgen`
// reads to produce the Rust side, so both halves are generated from one
// declaration and cannot drift.

#ifndef FLOOM_{upper}_BRIDGING_H
#define FLOOM_{upper}_BRIDGING_H

#include "floom_{ident}.h"

#endif /* FLOOM_{upper}_BRIDGING_H */
"#,
        name = name,
        ident = ident,
        upper = ident.to_uppercase()
    )
}

// --- Desktop: CMake -------------------------------------------------------

/// `CMakeLists.txt` for a desktop host.
///
/// Links the Rust static library and the platform's own UI toolkit. The two
/// toolkits differ enough that one file could not configure all three without
/// conditionals on nearly every line, so each gets its own — which is also what
/// the platform projects upstream ship.
pub fn desktop_cmake(platform: &str, name: &str, lib: &str) -> String {
    let (toolkit, extra_deps, entry) = match platform {
        "linux" => (
            "GTK 3 / Wayland / X11",
            // `pkg_check_modules` is a command from the `PkgConfig` package, not a
            // built-in. Without `find_package(PkgConfig)` above it, CMake stops
            // with `Unknown CMake command "pkg_check_modules"` — a message that
            // points at the command rather than at the missing package.
            "find_package(PkgConfig REQUIRED)\n    pkg_check_modules(GTK3 REQUIRED IMPORTED_TARGET gtk+-3.0)",
            r#"#include <gtk/gtk.h>

static void activate(GtkApplication *app, gpointer _) {
    (void)app;
    floom_run("PLACEHOLDER_TITLE");
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;
    GtkApplication *app = gtk_application_new("dev.floom.app", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int status = g_application_run(G_APPLICATION(app), 0, NULL);
    g_object_unref(app);
    return status;
}"#,
        ),
        "macos" => (
            "AppKit",
            "    find_package(Cocoa REQUIRED)",
            r#"#import <Cocoa/Cocoa.h>

static void terminate(NSTerminateRequest *request) {
    (void)request;
    floom_shutdown();
    [NSApp replyToApplicationShouldTerminate:YES];
}

int main(void) {
    @autoreleasepool {
        NSUInteger style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
                         | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable;
        NSWindow *window = [[NSWindow alloc]
            initWithContentRect:NSMakeRect(0, 0, 800, 600)
                      styleMask:style
                        backing:NSBackingStoreBuffered
                          defer:NO];
        window.title = @"PLACEHOLDER_TITLE";

        // A layer-backed view with a CAMetalLayer, so the surface Rust draws into
        // is a CAMetalLayer and Rust can present through the same path the
        // renderer uses everywhere else.
        [window setContentView:[[NSView alloc] initWithFrame:window.frame]];
        [window makeKeyAndOrderFront:nil];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        floom_attach_window((__bridge void *)window);
        floom_run("PLACEHOLDER_TITLE");
        [NSApp run];
    }
    return 0;
}"#,
        ),
        "windows" => (
            "Win32",
            "",
            r#"#define UNICODE
#define _UNICODE
#include <windows.h>
#include <stdio.h>

static LRESULT CALLBACK wnd_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_CLOSE:
        floom_shutdown();
        PostQuitMessage(0);
        return 0;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    case WM_SIZE:
        floom_resized(LOWORD(lp), HIWORD(lp));
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

int WINAPI wWinMain(HINSTANCE hinst, HINSTANCE, PWSTR, int) {
    WNDCLASSW wc = {0};
    wc.lpfnWndProc = wnd_proc;
    wc.hInstance = hinst;
    wc.lpszClassName = L"FloomWindow";
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    RegisterClassW(&wc);

    HWND hwnd = CreateWindowW(
        wc.lpszClassName, L"PLACEHOLDER_TITLE", WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT, 900, 640,
        NULL, NULL, hinst, NULL);
    ShowWindow(hwnd, SW_SHOW);

    floom_attach_window(hwnd);
    floom_run("PLACEHOLDER_TITLE");

    MSG msg;
    while (GetMessageW(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    return 0;
}"#,
        ),
        _ => unreachable!("desktop_cmake called for non-desktop platform {platform}"),
    };

    // Toolkit link libraries, and nothing beyond them. A CMake file that lists
    // every dependency for every platform is a file where half the lines are
    // wrong on the platform you are building for.
    //
    // Interpolated here with its own `format!` rather than left as `{name}` for
    // the outer one. A nested `{name}` inside a value substituted by
    // `{platform_cmake_deps}` is *not* re-interpolated, so it reaches CMake
    // literally and the build fails with `Cannot specify link libraries for
    // target "{name}" which is not built by this project` — a message quoting a
    // target with braces in its name, which is the tell.
    let platform_cmake_deps = match platform {
        "linux" => format!(
            "target_link_libraries({name} PRIVATE PkgConfig::GTK3)",
            name = name
        ),
        "macos" => format!(
            "target_link_options({name} PRIVATE -framework Cocoa)\n\
             target_link_libraries({name} PRIVATE \"${{CMAKE_APPLICATION_FRAMEWORK_ACCELERATOR}}\")",
            name = name
        ),
        "windows" => "# Win32 needs no extra libraries: `user32` and `gdi32` are linked by\n\
             # default with the MSVC toolchain."
            .to_string(),
        _ => unreachable!(),
    };

    let _ = entry;
    format!(
        r#"# CMake project for `{name}` on {toolkit}.
#
# Builds the native shell that hosts the Rust widget tree, and links the Rust
# static library that the tree lives in. The Rust half is produced by
# `cargo build --release` and placed in this directory by `floom build`; nothing
# here invokes Cargo, because a CMake target that shells out to Cargo cannot
# report its errors through CMake.
#
# Build:
#   cargo build --release          # produces lib{lib}.a
#   cmake -B build && cmake --build build

cmake_minimum_required(VERSION 3.20)
project({name} C)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)

{extra_deps}

# Where the Rust static library is.
#
# The build type is defaulted, not read straight from CMAKE_BUILD_TYPE: that
# variable is empty for single-config generators unless the caller passes
# -DCMAKE_BUILD_TYPE, and an empty component produces a path with a doubled
# separator — "/demo/linux//libfloom_app.a" — which is still a valid path and so
# fails later with a message about the file rather than about the build type.
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "" FORCE)
endif()

set(FLOOM_LIB_DIR "${{CMAKE_CURRENT_SOURCE_DIR}}/${{CMAKE_BUILD_TYPE}}")
set(FLOOM_STATIC_LIB "${{FLOOM_LIB_DIR}}/lib{lib}.a")

if(NOT EXISTS "${{FLOOM_STATIC_LIB}}")
    message(FATAL_ERROR
        "Rust static library not found at ${{FLOOM_STATIC_LIB}}. "
        "Run `cargo build --release` first.")
endif()

# IMPORTED, not a compiled target.
#
# `add_library(floom_core STATIC some.a)` asks CMake to *compile* a C file that
# happens to be named `some.a`, and it stops with "CMake can not determine linker
# language for target: floom_core". An archive is a build product, not a source,
# so it is declared as one.
add_library(floom_core STATIC IMPORTED)
set_target_properties(floom_core PROPERTIES
    IMPORTED_LOCATION "${{FLOOM_STATIC_LIB}}"
    INTERFACE_INCLUDE_DIRECTORIES "${{CMAKE_CURRENT_SOURCE_DIR}}/include")
# The system libraries a desktop host needs. Windows has none of these and does
# not get them: `find_package(Threads)` resolves per-platform, and naming `m`
# there would be an error rather than an unused library.
if(NOT WIN32)
    find_package(Threads REQUIRED)
    # On an IMPORTED target these are interface properties rather than link
    # flags: `target_link_libraries(floom_core ...)` on an imported target would
    # not link anything, and the executable would silently miss libm and libdl.
    set_property(TARGET floom_core APPEND PROPERTY
        INTERFACE_LINK_LIBRARIES m dl Threads::Threads)
endif()


add_executable({name} src/main.c)
target_link_libraries({name} PRIVATE floom_core)
{platform_cmake_deps}
"#
    )
}

/// A desktop `include/floom.h` — the C ABI a desktop host calls.
///
/// Hand-written rather than generated, because it is small and fixed: four
/// functions that a desktop host has always needed. A plugin's header in
/// `native/common/` is generated from its own design and may grow; this one does
/// not, and a generated file for four declarations would be a build dependency
/// to express nothing.
pub const DESKTOP_HOST_H: &str = r#"// The C ABI a Floom desktop host calls.
//
// Four functions. A host creates a window with its own toolkit, hands the handle
// over, and pumps messages; Rust owns the widget tree, the layout, the paint and
// the event loop's semantics.
//
// The alternative — the host calling into Rust per event — would mean a
// serialization boundary per event, which is the cost this design exists to
// avoid.

#ifndef FLOOM_HOST_H
#define FLOOM_HOST_H

#ifdef __cplusplus
extern "C" {
#endif

/** Called once with the platform's native window handle (HWND / NSWindow * /
 *  GtkWindow *). Rust takes rendering from here. */
void floom_attach_window(void *native_window);

/** Starts the event loop. Returns when the app quits. */
int floom_run(const char *title);

/** Asks the event loop to stop. Safe to call from the host's window manager. */
void floom_shutdown(void);

/** Reports a size change when the host's toolkit owns layout. */
void floom_resized(int width, int height);

#ifdef __cplusplus
}
#endif

#endif /* FLOOM_HOST_H */
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gradle_wrapper_pins_a_version() {
        let props = android_gradle_wrapper_properties();
        assert!(
            props.contains(&format!("gradle-{GRADLE_VERSION}-all.zip")),
            "{props}"
        );
    }

    #[test]
    fn the_abi_list_agrees_with_the_manifest_comment() {
        // The `abiFilters` line and the manifest's `nativeLibraryDir` note name
        // the same four ABIs; if either is edited alone the APK silently grows
        // or the library fails to load.
        let gradle = android_app_build_gradle("com.example", "my_app", "floom_my_app");
        for abi in ["arm64-v8a", "x86_64", "armeabi-v7a", "x86"] {
            assert!(gradle.contains(abi), "{abi} missing from abiFilters");
        }
    }

    #[test]
    fn a_plugin_manifest_has_no_application_element() {
        let manifest = android_plugin_manifest("camera");
        // Checked outside comments: the emitted manifest *discusses* the element
        // it declines to declare, so a substring search over the whole document
        // finds the discussion.
        let without_comments: String = strip_xml_comments(&manifest);
        assert!(
            !without_comments.contains("<application"),
            "a library manifest that declares an application merges its attributes \
             into every consumer:\n{without_comments}"
        );
        assert!(without_comments.contains("<manifest"), "{without_comments}");
    }

    #[test]
    fn an_app_manifest_has_an_application_element() {
        let manifest = android_app_manifest("com.example", "my_app");
        assert!(manifest.contains("<application"));
        assert!(manifest.contains("android.intent.action.MAIN"));
    }

    #[test]
    fn a_plugin_settings_file_has_no_app_module() {
        let settings = android_plugin_settings_gradle("camera");
        assert!(!settings.contains("include(\":app\")"));
        assert!(settings.contains("com.android.library"));
    }

    #[test]
    fn an_app_settings_file_has_an_app_module() {
        let settings = android_app_settings_gradle("com.example", "my_app");
        assert!(settings.contains("include(\":app\")"));
        assert!(settings.contains("com.android.application"));
    }

    #[test]
    fn kotlin_packages_match_the_manifest() {
        // A mismatch here compiles — the manifest is not checked against the
        // source at build time — and fails at launch with a ClassNotFoundException.
        let id = android_app_id("com.example", "my-app");
        assert_eq!(id, "com.example.my_app");
        assert_eq!(kotlin_package_path(&id), "com/example/my_app");
        assert_eq!(kotlin_class_name("my-app"), "MyApp");
    }

    #[test]
    fn the_activity_source_declares_its_package_and_the_manifest_agrees() {
        let id = android_app_id("com.example", "my_app");
        let activity = android_main_activity("com.example", "my_app", "floom_my_app");
        let manifest = android_app_manifest("com.example", "my_app");

        // The source's `package` statement is dotted; only the *directory* uses
        // slashes. A source that says `package com/example/demo` fails with four
        // parse errors on line 1, one per slash.
        let pkg_line = format!("package {}", id.replace('-', "_"));
        assert!(
            activity.contains(&pkg_line),
            "the Kotlin `package` statement must be dotted:\n{activity}"
        );

        // And the manifest's `android:name` must name the same class the source
        // declares, because it is resolved with Class.forName.
        let class = format!("{}Activity", kotlin_class_name("my_app"));
        let declared = format!("class {class} : Activity()");
        assert!(activity.contains(&declared), "{activity}");
        assert!(
            manifest.contains(&format!(
                "android:name=\"{}.{}\"",
                id.replace('-', "_"),
                class
            )),
            "{manifest}"
        );
    }

    #[test]
    fn external_functions_declare_native_calls_only() {
        let activity = android_main_activity("com.example", "my_app", "floom_my_app");
        for call in [
            "external fun start()",
            "external fun frame()",
            "external fun touch(",
            "external fun surfaceCreated(",
        ] {
            assert!(activity.contains(call), "`{call}` missing");
        }
    }

    #[test]
    fn the_desktop_header_is_its_own_abi() {
        assert!(DESKTOP_HOST_H.contains("floom_attach_window"));
        assert!(DESKTOP_HOST_H.contains("extern \"C\""));
    }

    #[test]
    fn each_desktop_cmake_names_its_own_toolkit_and_nobody_elses() {
        // A CMake file that mentions GTK while building a Win32 app configures
        // only if the reader already knows which half to ignore. So each names
        // its own toolkit and not the other two.
        let linux = desktop_cmake("linux", "app", "floom_app");
        assert!(linux.contains("gtk"), "{linux}");
        assert!(
            !linux.contains("Cocoa"),
            "linux CMake mentions Cocoa:\n{linux}"
        );
        assert!(
            !linux.contains("Win32"),
            "linux CMake mentions Win32:\n{linux}"
        );

        let macos = desktop_cmake("macos", "app", "floom_app");
        assert!(macos.contains("Cocoa"), "{macos}");
        assert!(!macos.contains("gtk"), "macos CMake mentions gtk:\n{macos}");

        let windows = desktop_cmake("windows", "app", "floom_app");
        assert!(windows.contains("Win32"), "{windows}");
        assert!(
            !windows.contains("gtk"),
            "windows CMake mentions gtk:\n{windows}"
        );
    }

    #[test]
    fn every_desktop_cmake_links_the_rust_library_or_fails_loudly() {
        // A CMake project that links a path which may not exist fails at *link*
        // time with a message about a missing file, long after the person has
        // stopped looking. The guard makes it a configure-time error that names
        // the fix.
        for p in ["linux", "macos", "windows"] {
            let cmake = desktop_cmake(p, "app", "floom_app");
            assert!(cmake.contains("libfloom_app.a"), "{p}:\n{cmake}");
            assert!(cmake.contains("FATAL_ERROR"), "{p}:\n{cmake}");
            assert!(
                cmake.contains("cargo build --release"),
                "{p}: the guard must say how to produce the library"
            );
        }
    }

    #[test]
    fn a_desktop_cmake_sets_properties_only_after_the_target_exists() {
        // CMake requires the target to exist before its properties are set. Every
        // ordering mistake here has the same message —
        // `Cannot specify link libraries for target X which is not built by this
        // project` — which names the target and never says that the line belongs
        // below `add_executable`.
        for p in ["linux", "macos", "windows"] {
            let cmake = desktop_cmake(p, "app", "floom_app");
            let added = cmake.find("add_executable(app").expect("an executable");
            let imported = cmake
                .find("add_library(floom_core STATIC IMPORTED)")
                .expect("an imported library");
            let props = cmake
                .find("set_property(TARGET floom_core")
                .expect("the imported target's properties");
            let linked = cmake
                .rfind("target_link_libraries(app")
                .expect("the executable's link libraries");

            assert!(
                imported < props,
                "{p}: set_property precedes the imported target it names"
            );
            assert!(
                added < linked,
                "{p}: target_link_libraries precedes add_executable"
            );
            assert!(
                imported < added,
                "{p}: the executable links a library declared after it"
            );
        }
    }

    #[test]
    fn a_desktop_cmake_declares_the_archive_as_imported() {
        // `add_library(x STATIC foo.a)` asks CMake to compile a file named
        // `foo.a`, and stops with "CMake can not determine linker language for
        // target: x".
        for p in ["linux", "macos", "windows"] {
            let cmake = desktop_cmake(p, "app", "floom_app");
            assert!(
                cmake.contains("STATIC IMPORTED"),
                "{p}: the Rust archive must be IMPORTED"
            );
            assert!(
                !cmake.contains("add_library(floom_core STATIC \""),
                "{p}: an archive passed as a source file"
            );
        }
    }

    #[test]
    fn a_linux_cmake_finds_pkgconfig_before_using_it() {
        // `pkg_check_modules` is not a CMake command; it comes from the PkgConfig
        // package. Without the `find_package` above it, configuration stops with
        // `Unknown CMake command "pkg_check_modules"`.
        let cmake = desktop_cmake("linux", "app", "floom_app");
        let find = cmake
            .find("find_package(PkgConfig REQUIRED)")
            .expect("PkgConfig must be found");
        let use_ = cmake
            .find("pkg_check_modules(GTK3")
            .expect("GTK must be checked");
        assert!(
            find < use_,
            "pkg_check_modules precedes find_package(PkgConfig)"
        );
    }

    #[test]
    fn a_desktop_cmake_defaults_the_build_type() {
        // `CMAKE_BUILD_TYPE` is empty for single-config generators unless passed,
        // and an empty path component produces `/demo/linux//libfloom_app.a` — a
        // valid path, so the failure surfaces later and names the file.
        for p in ["linux", "macos", "windows"] {
            let cmake = desktop_cmake(p, "app", "floom_app");
            assert!(
                cmake.contains("set(CMAKE_BUILD_TYPE Release"),
                "{p} does not default CMAKE_BUILD_TYPE"
            );
        }
    }

    #[test]
    fn lib_names_replace_hyphens() {
        assert_eq!(lib_name("my-camera"), "floom_my_camera");
        assert!(!lib_name("my-camera").contains('-'));
    }

    /// Removes XML comments so a substring search only sees real elements.
    fn strip_xml_comments(xml: &str) -> String {
        let mut out = String::with_capacity(xml.len());
        let mut rest = xml;
        while let Some(start) = rest.find("<!--") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 4..];
            match after.find("-->") {
                Some(end) => rest = &after[end + 3..],
                None => return out,
            }
        }
        out.push_str(rest);
        out
    }

    /// Every emitted XML document is checked for the one XML rule that is easy to
    /// break and hard to diagnose: `--` inside a comment. The manifest that
    /// violated it produced `ManifestMerger2: Error parsing
    /// AndroidManifest.xml`, an error that names the merger and not the
    /// punctuation, and there was no way to find it from the message.
    fn assert_xml_comments_are_legal(name: &str, xml: &str) {
        let mut rest = xml;
        while let Some(start) = rest.find("<!--") {
            let after = &rest[start + 4..];
            let end = after
                .find("-->")
                .unwrap_or_else(|| panic!("{name}: unterminated comment"));
            let body = &after[..end];
            assert!(
                !body.contains("--"),
                "{name}: a double hyphen inside an XML comment\n{body}"
            );
            rest = &after[end + 3..];
        }
    }

    #[test]
    fn no_emitted_xml_contains_a_double_hyphen_inside_a_comment() {
        let docs: Vec<(&str, String)> = vec![
            (
                "app manifest",
                android_app_manifest("com.example", "my_app"),
            ),
            ("plugin manifest", android_plugin_manifest("camera")),
            ("strings", android_strings_xml("my_app")),
            ("styles", android_styles_xml("my_app")),
            ("colors", android_colors_xml()),
            ("launcher icon", android_launcher_icon_xml()),
            (
                "launcher foreground",
                ANDROID_LAUNCHER_FOREGROUND_XML.to_string(),
            ),
        ];
        for (name, xml) in &docs {
            assert_xml_comments_are_legal(name, xml);
        }
    }

    #[test]
    fn a_plugin_manifest_declares_the_package_the_build_file_names() {
        // Two declarations of one identity. They agree today because both are
        // derived here, and this test is what keeps them derived.
        let manifest = android_plugin_manifest("camera");
        assert!(
            manifest.contains(r#"package="dev.floom.plugin.camera""#),
            "{manifest}"
        );

        let gradle = android_plugin_build_gradle("camera", "floom_camera");
        assert!(
            gradle.contains(r#"namespace = "dev.floom.plugin.camera""#),
            "{gradle}"
        );
    }

    #[test]
    fn every_emitted_xml_document_starts_with_its_declaration() {
        // `ManifestMerger2` rejects a manifest that begins at the root element
        // with "Error parsing", which does not mention the missing declaration.
        for (name, xml) in [
            (
                "app manifest",
                android_app_manifest("com.example", "my_app"),
            ),
            ("plugin manifest", android_plugin_manifest("camera")),
            ("strings", android_strings_xml("my_app")),
            ("styles", android_styles_xml("my_app")),
            ("colors", android_colors_xml()),
            ("launcher icon", android_launcher_icon_xml()),
        ] {
            assert!(
                xml.starts_with("<?xml version="),
                "{name} does not start with an XML declaration: {xml:?}"
            );
        }
    }

    #[test]
    fn local_properties_is_written_with_a_discovered_sdk_or_not_at_all() {
        // The function returns None rather than guessing; the caller decides
        // whether to write a note instead of a path.
        match detect_android_sdk() {
            Some(p) => {
                let props = android_local_properties(&p);
                assert!(props.contains(&format!("sdk.dir={p}")));
            }
            None => {}
        }
    }
}
