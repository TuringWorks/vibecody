import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "dev.vibecody.vibecody_mobile"
    // 37, not flutter.compileSdkVersion: permission_handler_android (pulled in
    // by the voice-input microphone permission added in v0.5.8) declares an AAR
    // metadata minimum of API 37, and `:app:checkReleaseAarMetadata` fails the
    // release build against anything lower. compileSdk only controls which APIs
    // are available at compile time; minSdk/targetSdk are unchanged, so device
    // support is unaffected.
    compileSdk = 37
    ndkVersion = "28.2.13676358"

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "dev.vibecody.vibecody_mobile"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

// Outside `android { }` on purpose. AGP 9 deprecates `kotlinOptions` at ERROR
// level, and because that block only exists on the legacy BaseAppModuleExtension,
// leaving it in also pins `android { }` to the deprecated overload — which is why
// one stale block produced three "script compilation errors" here. The
// compilerOptions DSL is the AGP 9 replacement.
kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_11
    }
}

flutter {
    source = "../.."
}

dependencies {
    // Wear OS Data Layer — receives relay requests from VibeCodyWear when
    // the watch has no direct network. See:
    //   vibemobile/android/app/src/main/kotlin/.../wear/WearDataLayerService.kt
    implementation("com.google.android.gms:play-services-wearable:20.0.1")
    // OkHttp powers the relay HTTP forwarding inside WearDataLayerService.
    implementation("com.squareup.okhttp3:okhttp:5.4.0")
}
