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
# The notification permission given (its prompt would be in front of the app).
adb shell pm grant "$pkg" android.permission.POST_NOTIFICATIONS
# The install's broadcasts settle first (one arriving after the start restarts the activity).
sleep 20
adb shell wm size > "$out/screen.txt"
adb shell wm density >> "$out/screen.txt"
# MDM's lab server on the runner, reached from the emulator at 10.0.2.2: a 100 MB file at 1 MB/s
# a connection, so the download is still running for the first screenshots.
target/release/labserver --bind 0.0.0.0:8090 --log-file "$out/lab.jsonl" \
  > /dev/null 2> "$out/lab.err" &
lab=$!
sleep 2
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
# The language page's x (its top bar's first button) closes it. (Back would first hide the
# keyboard its Search box opens.)
adb shell input tap $(dp 28) $top
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
shot 05b-more-closed
# Add URL (the bottom bar's first of four buttons), an address typed in it.
adb shell input tap 135 $bottom
shot 06-add-url
adb shell input text "'http://10.0.2.2:8090/gen/100M/100Mb.dat?rate=1M'"
shot 07-typed
# Enter: OK (Add URL's default button), then Download File Info's Start Download.
adb shell input keyevent KEYCODE_ENTER
sleep 6
shot 08-file-info
adb shell input keyevent KEYCODE_ENTER
sleep 4
shot 09-downloading
# The background notification, in the shade.
adb shell cmd statusbar expand-notifications
shot 09b-notification
adb shell cmd statusbar collapse
sleep 10
shot 09c-later
# The download done (its .part renamed), up to three minutes, then its bytes checked.
dir=/sdcard/Download/MisterDownloadManager
for _ in $(seq 90); do
  adb shell ls "$dir/100Mb.dat" > /dev/null 2>&1 && break
  sleep 2
done
shot 09d-done
# Open with...: the row's menu (a long press, held 2 s: the emulator draws a frame in up to a
# second, and egui sees the press as long from 0.8 s), its second entry, which hands the file to
# Android's chooser of apps. Back returns to MDM.
adb shell input swipe 540 305 540 305 2000
shot 09e-row-menu
adb shell input tap 540 605
shot 09f-open-with
adb shell input keyevent KEYCODE_BACK
sleep 2
adb shell ls -la "$dir" > "$out/downloads.txt" 2>&1 || true
adb shell cat "$dir/*.mdm.json" >> "$out/downloads.txt" 2>&1 || true
{
  echo "device: $(adb shell sha256sum "$dir/100Mb.dat" 2>&1 | cut -d' ' -f1)"
  echo "server: $(curl -s http://127.0.0.1:8090/gen/100M/100Mb.dat | sha256sum | cut -d' ' -f1)"
} > "$out/sha256.txt"
# Back on the main screen, nothing in front: the app goes to the background (the launcher shows),
# then comes back as it was.
adb shell input keyevent KEYCODE_BACK
shot 10-back-home
adb shell dumpsys activity activities | grep -E "topResumedActivity|mResumedActivity" > "$out/after-back.txt"
adb shell am start -W -n "$pkg/com.misterdownloadmanager.android.MainActivity"
shot 10b-back-again
# Options: the more menu's Options (146 dp above the screen's content bottom), its list of
# sections, Save to (the third), then Back to the list and Back to close it.
adb shell input tap $(( 1080 - $(dp 28) )) $top
sleep 2
adb shell input tap 540 1890
shot 10c-options
adb shell input tap 540 572
shot 10d-save-to
adb shell input keyevent KEYCODE_BACK
shot 10e-options-again
adb shell input keyevent KEYCODE_BACK
sleep 2
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
kill $lab 2>/dev/null
exit 0
