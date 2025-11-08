# app_egui

## Android

### Dependencies 

- [Android Studio](https://developer.android.com/studio) needs to be installed, along with NDK and an SDK supporting a minimum of API 26.
- Environment variables need to be configured for `ANDROID_HOME` and `ANDROID_NDK_ROOT`.
- `cargo-apk` is currently used to build an APK for the app.
  - To install `cargo-apk`, run `cargo install cargo-apk`.

### Building + Running

- `cargo apk build -p app_egui --lib` to build the app.
  - Append `--release` for a release build.
- `cargo apk run -p app_egui --lib` to run the app in a connected device.
