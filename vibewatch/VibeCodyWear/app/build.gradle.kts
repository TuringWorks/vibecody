plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vibecody.wear"
    compileSdk = 36          // Android 16 / Wear OS 6

    defaultConfig {
        applicationId = "com.vibecody.wear"
        minSdk = 30          // Wear OS 3.0
        targetSdk = 36       // Android 16 / Wear OS 6
        versionCode = 3
        versionName = "0.5.7"
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

    lint {
        // androidx.lifecycle's NonNullableMutableLiveDataDetector crashes under
        // AGP 8.7.3's bundled lint with an IncompatibleClassChangeError
        // (KaCallableMemberCall class-vs-interface — a Kotlin analysis-API
        // version skew), aborting lintVitalAnalyzeRelease and thus the release
        // build. This is a Compose app with no LiveData usage, so the check has
        // nothing to verify here — disable it to unblock assembleRelease.
        disable += "NullSafeMutableLiveData"
    }
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
