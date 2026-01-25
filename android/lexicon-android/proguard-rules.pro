# Lexicon FFI ProGuard Rules

# Keep UniFFI generated classes
-keep class uniffi.lexicon.** { *; }

# Keep JNA classes
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep Lexicon wrapper classes
-keep class com.lexicon.core.** { *; }
