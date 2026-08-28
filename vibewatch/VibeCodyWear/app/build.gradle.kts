import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vibecody.wear"
    // compileSdk only widens the API surface available at compile time;
    // runtime behaviour still follows targetSdk below. androidx.lifecycle
    // 2.11.0 refuses to link against anything older than 37.
    compileSdk = 37          // Android 17

    defaultConfig {
        applicationId = "com.vibecody.wear"
        minSdk = 30          // Wear OS 3.0
        targetSdk = 36       // Android 16 / Wear OS 6
        versionCode = 6
        versionName = "0.5.11"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures { compose = true }
}

// AGP 9 supplies the Kotlin plugin itself, so the compiler options move out of
// android{} (where `kotlinOptions` no longer exists) into the Kotlin extension.
kotlin {
    compilerOptions { jvmTarget = JvmTarget.JVM_17 }
}

dependencies {
    // Wear OS Compose
    implementation(libs.androidx.wear.compose.material)
    implementation(libs.androidx.wear.compose.foundation)
    implementation(libs.androidx.wear.compose.navigation)

    // Tiles + Complications
    implementation(libs.androidx.wear.tiles)
    implementation(libs.androidx.wear.tiles.material)
    // Tile services return ListenableFuture and use CallbackToFutureAdapter
    implementation(libs.guava)
    implementation(libs.androidx.concurrent.futures)
    // @Preview annotation used by RecapScreen
    implementation(libs.androidx.compose.ui.tooling.preview)

    // Activity + Lifecycle
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)

    // Wearable Data Layer (phone relay)
    implementation(libs.play.services.wearable)

    // Network — OkHttp (SSE) + Moshi
    implementation(libs.okhttp)
    implementation(libs.okhttp.sse)
    implementation(libs.moshi.kotlin)

    // Security — EncryptedSharedPreferences
    implementation(libs.androidx.security.crypto)

    // Coroutines
    implementation(libs.kotlinx.coroutines.play.services)
}
