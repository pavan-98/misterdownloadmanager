#!/usr/bin/env bash
# Not for the pull request (android-try.yml): installs the debug APK on the running emulator,
# starts it, and keeps screenshots and the log in try-out/.
set -u
out=try-out
mkdir -p "$out"
pkg=com.misterdownloadmanager.android.debug
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb shell wm size > "$out/screen.txt"
adb shell wm density >> "$out/screen.txt"
adb logcat -c
adb shell am start -W -n "$pkg/com.misterdownloadmanager.android.MainActivity"
shot() { sleep 3; adb exec-out screencap -p > "$out/$1.png"; }
sleep 25
shot 01-start
# Back: the language window closes (Back is Esc), the main window shows.
adb shell input keyevent KEYCODE_BACK
shot 02-back
# A tap on the toolbar's first button (Add URL), at the top left under the status bar.
adb shell input tap 80 260
shot 03-add-url
adb shell input text "https://example.com/file.zip"
shot 04-typed
adb shell input keyevent KEYCODE_BACK
shot 05-back-again
adb shell pidof "$pkg" > "$out/pid.txt" || echo "not running" > "$out/pid.txt"
adb logcat -d > "$out/logcat.txt"
adb shell run-as "$pkg" cat files/mdm/logs/mdm.log > "$out/mdm.log" 2>/dev/null || true
adb shell run-as "$pkg" cat files/mdm/crash.log > "$out/crash.log" 2>/dev/null || true
exit 0
