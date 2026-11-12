#!/bin/bash
set -e

apt-get update -qq && apt-get install -y -qq wget unzip 2>&1 | tail -5

mkdir -p /opt/android-sdk && cd /opt/android-sdk
wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O cmdtools.zip
unzip -q cmdtools.zip -d cmdline-tools

# The zip extracts to cmdline-tools/cmdline-tools/
export ANDROID_HOME=/opt/android-sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/cmdline-tools/bin:$ANDROID_HOME/platform-tools

yes | sdkmanager --licenses >/dev/null 2>&1
sdkmanager 'platforms;android-34' 'build-tools;34.0.0' 'platform-tools' >/dev/null 2>&1

cd /workspace && ./gradlew assembleDebug --no-daemon