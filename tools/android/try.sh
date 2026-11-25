#!/usr/bin/env bash
# Not for the pull request (android-try.yml): installs the debug APK on the running emulator,
# starts it, and keeps screenshots and the log in try-out/.
set -u
out=try-out
mkdir -p "$out"
pkg=com.misterdownloadmanager.android.debug
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb logcat -c
adb shell am start -W -n "$pkg/com.misterdownloadmanager.android.MainActivity"
shot() { adb exec-out screencap -p > "$out/$1.png"; }
sleep 20
shot 01-start
adb shell dumpsys window | grep -E "mCurrentFocus|mFocusedApp" > "$out/focus.txt" || true
# A tap in the middle (the language window's list), then back.
adb shell input tap 540 1200
sleep 3
shot 02-tap
adb shell input keyevent KEYCODE_BACK
sleep 3
shot 03-back
adb shell pidof "$pkg" > "$out/pid.txt" || echo "not running" > "$out/pid.txt"
adb logcat -d > "$out/logcat.txt"
adb shell run-as "$pkg" cat files/mdm/logs/mdm.log > "$out/mdm.log" 2>/dev/null || true
adb shell run-as "$pkg" cat files/mdm/crash.log > "$out/crash.log" 2>/dev/null || true
exit 0
