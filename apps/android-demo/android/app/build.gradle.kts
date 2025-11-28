plugins {
    id("com.android.application")
}

android {
    namespace = "com.compose_rs.demo"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.compose_rs.demo"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        debug {
            // Debug builds use only arm64-v8a for faster iteration
            // Match this to your dev device ABI
            ndk {
                abiFilters.add("arm64-v8a")
            }
        }
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )

            // Release builds include all supported ABIs
            ndk {
                abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
            }
        }
    }

    // Enable ABI splits for release builds to generate separate APKs per ABI
    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
            isUniversalApk = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    sourceSets {
        getByName("debug") {
            jniLibs.srcDirs("../../../target/android/debug")
        }
        getByName("release") {
            jniLibs.srcDirs("../../../target/android/release")
        }
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.6.1")
}

// Check if cargo-ndk is available
fun checkCargoNdk() {
    try {
        exec {
            commandLine("cargo", "ndk", "--version")
            standardOutput = java.io.OutputStream.nullOutputStream()
            errorOutput = java.io.OutputStream.nullOutputStream()
        }
    } catch (e: Exception) {
        throw GradleException(
            "cargo-ndk is not installed. Install it with:\n" +
            "    cargo install cargo-ndk\n" +
            "See: https://github.com/bbqsrc/cargo-ndk"
        )
    }
}

// Task to build Rust library for Android debug builds
tasks.register<Exec>("buildRustDebug") {
    description = "Build Rust library for Android (debug, single ABI)"
    group = "rust"

    // Check cargo-ndk availability
    doFirst {
        checkCargoNdk()
    }

    workingDir = rootProject.projectDir

    environment("CARGO_TARGET_DIR", "${rootProject.projectDir}/target/android/debug")

    // Debug builds: single ABI for faster iteration
    // Adjust arm64-v8a to match your dev device
    commandLine("sh", "-c", """
        cargo ndk \
            -t arm64-v8a \
            -o target/android/debug \
            build \
            -p desktop-app \
            --lib \
            --features android,renderer-wgpu \
            --no-default-features
    """)
}

// Task to build Rust library for Android release builds
tasks.register<Exec>("buildRustRelease") {
    description = "Build Rust library for Android (release, all ABIs)"
    group = "rust"

    // Check cargo-ndk availability
    doFirst {
        checkCargoNdk()
    }

    workingDir = rootProject.projectDir

    environment("CARGO_TARGET_DIR", "${rootProject.projectDir}/target/android/release")

    // Release builds: all supported ABIs
    commandLine("sh", "-c", """
        cargo ndk \
            -t arm64-v8a \
            -t armeabi-v7a \
            -t x86 \
            -t x86_64 \
            -o target/android/release \
            build \
            --release \
            -p desktop-app \
            --lib \
            --features android,renderer-wgpu \
            --no-default-features
    """)
}

// Wire Rust builds to Android build variants
tasks.named("preDebugBuild") {
    dependsOn("buildRustDebug")
}

tasks.named("preReleaseBuild") {
    dependsOn("buildRustRelease")
}
