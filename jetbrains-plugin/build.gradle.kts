import org.jetbrains.intellij.platform.gradle.TestFrameworkType

plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.24"
    id("org.jetbrains.intellij.platform") version "2.1.0"
}

group = "com.vibecody"
version = providers.gradleProperty("pluginVersion").get()

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        create(
            providers.gradleProperty("platformType"),
            providers.gradleProperty("platformVersion"),
        )
        bundledPlugins("com.intellij.java")
        pluginVerifier()
        zipSigner()
        testFramework(TestFrameworkType.Platform)
    }

    // JSON parsing for daemon API responses
    implementation("com.google.code.gson:gson:2.10.1")

    testImplementation("junit:junit:4.13.2")
}

intellijPlatform {
    pluginConfiguration {
        id = "com.vibecody.vibecli"
        name = "VibeCLI"
        version = providers.gradleProperty("pluginVersion")
        description = """
            AI coding assistant powered by VibeCLI.

            Connects to a running <code>vibecli serve</code> daemon to provide:
            <ul>
              <li>Chat with AI directly from the IDE</li>
              <li>Run agentic tasks (file edits, shell commands, multi-step plans)</li>
              <li>Inline AI edits — select code, press Ctrl+Shift+K, describe the change</li>
            </ul>

            <b>Quick start</b>
            <ol>
              <li>Start the daemon: <code>vibecli serve --port 7878</code></li>
              <li>Configure daemon URL in VibeCLI settings (default: http://localhost:7878)</li>
              <li>Open the VibeCLI tool window (View → Tool Windows → VibeCLI)</li>
            </ol>
        """.trimIndent()

        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
            untilBuild = providers.gradleProperty("pluginUntilBuild")
        }

        vendor {
            name = "Vibe Team"
            url = "https://github.com/vibecody/vibecody"
        }

        changeNotes = """
            <b>0.1.0</b>
            <ul>
              <li>Initial release</li>
              <li>Chat panel with streaming responses</li>
              <li>Agent task submission and progress streaming</li>
              <li>Inline edit action (Ctrl+Shift+K)</li>
              <li>Configurable daemon URL and API provider</li>
            </ul>
        """.trimIndent()
    }

    signing {
        certificateChain = providers.environmentVariable("CERTIFICATE_CHAIN")
        privateKey = providers.environmentVariable("PRIVATE_KEY")
        password = providers.environmentVariable("PRIVATE_KEY_PASSWORD")
    }

    publishing {
        token = providers.environmentVariable("PUBLISH_TOKEN")
        channels = listOf(
            if (providers.gradleProperty("pluginVersion").get().contains("beta"))
                "beta" else "default"
        )
    }

    pluginVerification {
        ides {
            recommended()
        }
    }
}

kotlin {
    jvmToolchain(17)
}

tasks {
    wrapper {
        gradleVersion = "8.8"
    }
}
