#!/bin/bash
set -e

apt-get update -qq && apt-get install -y openjdk-17-jdk-headless wget unzip

wget -q https://services.gradle.org/distributions/gradle-8.5-bin.zip -O /tmp/gradle.zip
unzip -q /tmp/gradle.zip -d /opt/gradle
export PATH=$PATH:/opt/gradle/gradle-8.5/bin

mkdir -p /opt/android-sdk && cd /opt/android-sdk
wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O cmdtools.zip
unzip -q cmdtools.zip -d cmdline-tools

export ANDROID_HOME=/opt/android-sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/cmdline-tools/bin:$ANDROID_HOME/platform-tools

yes | sdkmanager --licenses >/dev/null 2>&1
sdkmanager 'platforms;android-34' 'build-tools;34.0.0' 'platform-tools'

cd /workspace
gradle wrapper --gradle-version 8.5 --distribution-type all
./gradlew assembleDebug --no-daemon