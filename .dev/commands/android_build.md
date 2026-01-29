# Android Build Process

***Paths relative to ./Lexicon-Local-Server working directory***
***Lexicon-Mobile required as sibling directory***

## 1. Rust Build

### Build for x86_64

rustup target add x86_64-linux-android
./scripts/android/build-android.sh x86_64 --release

### Build for aarch64/ARM64

rustup target add aarch64-linux-android
./scripts/android/build-android.sh x86_64 --release

### Build for x86

rustup target add i686-linux-android
./scripts/android/build-android.sh x86 --release

### Build for arm32

rustup target add armv7-linux-androideabi
./scripts/android/build-android.sh arm32 --release

## 2. AAR Build

cd android && ./gradlew clean assembleRelease

export JAVA_HOME="C:/Program Files/Android/Android Studio/jbr"
export PATH="$JAVA_HOME/bin:$PATH"
./gradlew clean assembleRelease

## 3. Copy AAR to Mobile app/libs/

cp "N:/repos/Lexicon-Suite/Lexicon-Local-Server/android/lexicon-android/build/outputs/aar/lexicon-android-release.aar" "N:/repos/Lexicon-Suite/Lexicon-Mobile/app/libs"

## 4. Move FFI Libraries to Mobile app/libs/

### x86_64 FFI Copy

mkdir -p android/lexicon-android/src/main/jniLibs/x86_64

cp target/x86_64-linux-android/release/liblexicon_ffi.so \
   "N:/repos/Lexicon-Suite/Lexicon-Mobile/app/libs/x86_64/"

### aarch64/ARM64 FFI Copy

cp target/aarch64-linux-android/release/liblexicon_ffi.so \
   "N:/repos/Lexicon-Suite/Lexicon-Mobile/app/libs/aarch64/"

### x86 FFI Copy

cp target/i686-linux-android/release/liblexicon_ffi.so \
   "N:/repos/Lexicon-Suite/Lexicon-Mobile/app/libs/x86/"

### arm32 FFI Copy

cp target/armv7-linux-androideabi/release/liblexicon_ffi.so \
   "N:/repos/Lexicon-Suite/Lexicon-Mobile/app/libs/arm32/"

## 5. Copy Test Audio

"N:/.DEV/tools/Android/platform-tools/adb" push "N:/repos/Lexicon-Suite/Lexicon-Local-Server/tests/fixtures/audio/woodchuck.wav" /sdcard/Download/
