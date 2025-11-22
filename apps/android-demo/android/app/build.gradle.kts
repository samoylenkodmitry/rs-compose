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

        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64"))
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("../../../target/android")
        }
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.6.1")
}

// Task to build Rust library
tasks.register<Exec>("buildRustAndroid") {
    workingDir = file("../../")

    doFirst {
        // Build for all Android ABIs
        val abis = listOf(
            "aarch64-linux-android" to "arm64-v8a",
            "armv7-linux-androideabi" to "armeabi-v7a",
            "i686-linux-android" to "x86",
            "x86_64-linux-android" to "x86_64"
        )

        for ((target, abi) in abis) {
            exec {
                commandLine("cargo", "ndk", "-t", abi, "-o", "../target/android", "build", "--release")
                workingDir = file("../../")
            }
        }
    }
}

tasks.named("preBuild") {
    dependsOn("buildRustAndroid")
}
