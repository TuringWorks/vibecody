plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vibecody.wear"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.vibecody.wear"
        minSdk = 30          // Wear OS 3.0
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
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
    kotlinOptions { jvmTarget = "17" }
    buildFeatures { compose = true }
}

dependencies {
    // Wear OS Compose
    implementation(libs.androidx.wear.compose.material)
    implementation(libs.androidx.wear.compose.foundation)
    implementation(libs.androidx.wear.compose.navigation)

    // Tiles + Complications
    implementation(libs.androidx.wear.tiles)
    implementation(libs.androidx.wear.tiles.material)

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

    // Security — Android Keystore (StrongBox)
    // No external dep needed; uses system APIs

    // Coroutines
    implementation(libs.kotlinx.coroutines.play.services)

    // Speech recognition
    implementation(libs.androidx.speech.recognizer) // on-device
}
