# apps/android/app/proguard-rules.pro
# Keep UniFFI-generated bindings: they use JNA reflection and call native symbols.
-dontwarn com.sun.jna.**
-keep class com.sun.jna.** { *; }
-keep class com.tongpin.todo.uniffi.** { *; }
-keepclassmembers class * extends com.sun.jna.** { *; }
