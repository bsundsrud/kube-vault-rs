#!/bin/sh
is_osx() {
    case "$TRAVIS_OS_NAME" in
        osx) return 0 ;;
        *)   return 1 ;;
    esac
}

cd ..
if is_osx; then
    make release
else
    make release-static
fi
make dist VERSION="$TRAVIS_TAG"
