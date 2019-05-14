#!/bin/sh
is_osx() {
    case "$TRAVIS_OS_NAME" in
        osx) return 0 ;;
        *)   return 1 ;;
    esac
}

if is_osx; then
    make release
else
    rustup add target x86_64-unknown-linux-musl
    make release-musl
fi
make dist VERSION="$TRAVIS_TAG"
