# Android Demo App

This is the Android demo application for Compose-RS, showcasing the cross-platform UI framework running natively on Android devices.

## Prerequisites

Before building the Android app, you need to install:

1. **Rust and Cargo** - Install from [rustup.rs](https://rustup.rs/)
2. **Android NDK** - Install via Android Studio SDK Manager or standalone
3. **cargo-ndk** - Install with: `cargo install cargo-ndk`
4. **Android SDK** - Install via Android Studio
5. **Gradle** - Usually comes with Android Studio

### Setting up Android targets

Add the required Rust targets for Android:

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android
```

### Environment Variables

Set the following environment variables (adjust paths as needed):

```bash
export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125  # Adjust version as needed
export PATH=$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/tools
```

## Opening in Android Studio

The easiest way to build and run the Android app is to open it in Android Studio:

1. Open Android Studio
2. Click "Open an Existing Project"
3. Navigate to and select the `apps/android-demo/android` directory
4. Wait for Gradle sync to complete (Android Studio will automatically download dependencies)
5. Click the Run button or press Shift+F10

Android Studio will automatically:
- Download the Gradle wrapper JAR
- Sync Gradle dependencies
- Build the Rust library using cargo-ndk (if you have the prerequisites installed)

## Building from Command Line

### Option 1: Build with Gradle (Recommended)

The Gradle build will automatically build the Rust library for all Android ABIs:

```bash
cd android
./gradlew assembleDebug
```

The APK will be generated at: `android/app/build/outputs/apk/debug/app-debug.apk`

**Note:** If `gradlew` fails the first time, run it again - it will download the Gradle wrapper JAR on the first run.

### Option 2: Build Rust Library Manually

If you want to build the Rust library separately:

```bash
# Build for all ABIs
cargo ndk -o ./target/android -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 build --release

# Or build for a specific ABI
cargo ndk -o ./target/android -t arm64-v8a build --release
```

Then build the Android app:

```bash
cd android
./gradlew assembleDebug
```

## Installing and Running

### Install on Device/Emulator

```bash
cd android
./gradlew installDebug
```

Or use adb directly:

```bash
adb install android/app/build/outputs/apk/debug/app-debug.apk
```

### Run from Android Studio

See the "Opening in Android Studio" section above for the complete workflow.

## Features

The Android demo includes:

- **Counter Example**: Interactive counter demonstrating state management
- **UI Components**: Buttons, text, layouts, and styling
- **Cross-platform Code**: Same Rust code runs on Desktop and Android

## Architecture

The Android app uses:

- **NativeActivity**: Android's native activity for Rust-only apps
- **NDK**: For native code integration
- **WGPU**: For hardware-accelerated rendering (when available)
- **Compose-RS**: The declarative UI framework

## Troubleshooting

### Build Fails

- Ensure all Rust targets are installed
- Check that ANDROID_NDK_HOME is set correctly
- Verify NDK version is compatible (26.x recommended)

### App Crashes on Launch

- Check logcat: `adb logcat | grep ComposeRS`
- Ensure the correct ABI is built for your device/emulator
- Try building in release mode for better performance

### Gradle Sync Issues

- Check that ANDROID_HOME points to your SDK installation
- Ensure Gradle version is compatible (8.1+ recommended)
- Clear Gradle cache: `./gradlew clean`

## Development

To modify the app:

1. Edit Rust code in `src/lib.rs`
2. Rebuild with `./gradlew assembleDebug` or cargo-ndk
3. Install and test on device/emulator

## License

This project is available under the Apache License (Version 2.0).
