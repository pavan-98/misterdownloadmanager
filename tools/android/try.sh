#!/usr/bin/env bash
# Not for the pull request (android-try.yml): installs the debug APK on the running emulator
# (a Pixel 6: 1080 × 2400 px at 2.625 px per dp), starts it, walks through the phone layout, and
# keeps screenshots and the log in try-out/.
set -u
out=try-out
mkdir -p "$out"
pkg=com.misterdownloadmanager.android.debug
# The system fully up (its BOOT_COMPLETED sent, the setup wizard gone) before the app starts.
until [ "$(adb shell getprop sys.boot_completed | tr -d '\r')" = "1" ]; do sleep 2; done
sleep 30
adb shell input keyevent KEYCODE_HOME
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
# The install's broadcasts settle first (one arriving after the start restarts the activity).
sleep 20
adb shell wm size > "$out/screen.txt"
adb shell wm density >> "$out/screen.txt"
adb logcat -c
adb shell am start -W -n "$pkg/com.misterdownloadmanager.android.MainActivity"
shot() { sleep 3; adb exec-out screencap -p > "$out/$1.png"; }
dp() { echo $(( $1 * 2625 / 1000 )); }
# Under the status bar (24 dp), the top bar is 56 dp; the bottom bar sits above the navigation
# bar (48 dp) at the screen's bottom.
top=$(( $(dp 24) + $(dp 28) ))
bottom=$(( 2400 - $(dp 48) - $(dp 28) ))
sleep 25
shot 01-start
# Back: the language window closes (Back is Esc).
adb shell input keyevent KEYCODE_BACK
shot 02-main
# The drawer (the top bar's first button), then Back closes it.
adb shell input tap $(dp 28) $top
shot 03-drawer
adb shell input keyevent KEYCODE_BACK
shot 04-drawer-closed
# The "more" menu (the top bar's last button), then Back.
adb shell input tap $(( 1080 - $(dp 28) )) $top
shot 05-more
adb shell input keyevent KEYCODE_BACK
# Add URL (the bottom bar's first of four buttons), an address typed in it.
adb shell input tap 135 $bottom
shot 06-add-url
adb shell input text "https://proof.ovh.net/files/1Mb.dat"
shot 07-typed
# Enter: OK (Add URL's default button), then Download File Info's Start Download.
adb shell input keyevent KEYCODE_ENTER
sleep 6
shot 08-file-info
adb shell input keyevent KEYCODE_ENTER
sleep 15
shot 09-downloaded
adb shell input keyevent KEYCODE_BACK
shot 10-closed
# The activity destroyed as the system may destroy it ("Don't keep activities"), then opened
# again: the app closes (saving its list) and starts again in the same process.
adb shell settings put global always_finish_activities 1
adb shell input keyevent KEYCODE_HOME
sleep 8
adb shell am start -W -n "$pkg/com.misterdownloadmanager.android.MainActivity"
sleep 15
shot 11-reopened
adb shell settings put global always_finish_activities 0
adb shell pidof "$pkg" > "$out/pid.txt" || echo "not running" > "$out/pid.txt"
adb logcat -d > "$out/logcat.txt"
adb shell run-as "$pkg" cat files/mdm/logs/mdm.log > "$out/mdm.log" 2>/dev/null || true
adb shell run-as "$pkg" cat files/mdm/crash.log > "$out/crash.log" 2>/dev/null || true
exit 0
