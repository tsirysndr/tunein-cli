#!/bin/bash

readonly MAGENTA="$(tput setaf 5 2>/dev/null || echo '')"
readonly GREEN="$(tput setaf 2 2>/dev/null || echo '')"
readonly CYAN="$(tput setaf 6 2>/dev/null || echo '')"
readonly NO_COLOR="$(tput sgr0 2>/dev/null || echo '')"

# Define the release information
RELEASE_URL="https://api.github.com/repos/tsirysndr/tunein/releases/latest"

# Determine the operating system
OS=$(uname -s)
if [ "$OS" = "Darwin" ]; then
    # Determine the CPU architecture
    ARCH=$(uname -m)
    if [ "$ARCH" = "arm64" ]; then
        ASSET_NAME="_aarch64-apple-darwin.tar.gz"
    else
        ASSET_NAME="_x86_64-apple-darwin.tar.gz"
    fi
elif [ "$OS" = "Linux" ]; then
    # Determine the CPU architecture
    ARCH=$(uname -m)
    if [ "$ARCH" = "aarch64" ]; then
        ASSET_NAME="_aarch64-unknown-linux-gnu.tar.gz"
    elif [ "$ARCH" = "x86_64" ]; then
        ASSET_NAME="_x86_64-unknown-linux-gnu.tar.gz"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported operating system: $OS"
    exit 1
fi

# Retrieve the download URL for the desired asset
DOWNLOAD_URL=$(curl -sSL $RELEASE_URL | grep -o "browser_download_url.*$ASSET_NAME\"" | cut -d ' ' -f 2)

ASSET_NAME=$(basename $DOWNLOAD_URL)

# Define the installation directory
INSTALL_DIR="/usr/local/bin"

DOWNLOAD_URL=`echo $DOWNLOAD_URL | tr -d '\"'`

# Download the asset
curl -SL $DOWNLOAD_URL -o /tmp/$ASSET_NAME

# Extract the asset
tar -xzf /tmp/$ASSET_NAME -C /tmp

# Set the correct permissions for the binary
chmod +x /tmp/tunein

# Move the extracted binary to the installation directory
# use sudo if OS is Linux
if [ "$OS" = "Linux" ]; then
    if command -v sudo >/dev/null 2>&1; then
        sudo mv /tmp/tunein $INSTALL_DIR
    else
        mv /tmp/tunein $INSTALL_DIR
    fi
else
    mv /tmp/tunein $INSTALL_DIR
fi

# Clean up temporary files
rm /tmp/$ASSET_NAME

cat << EOF
${CYAN}
        ______              ____       _______   ____
       /_  __/_ _____  ___ /  _/__    / ___/ /  /  _/
        / / / // / _ \\/ -_)/ // _ \\  / /__/ /___/ /  
       /_/  \\_,_/_//_/\\__/___/_//_/  \\___/____/___/  
                                                              
${NO_COLOR}
Browse and listen to thousands of radio stations across the globe right from your terminal 🌎 📻 🎵✨

${GREEN}https://github.com/tsirysndr/tunein${NO_COLOR}

Please file an issue if you encounter any problems!

===============================================================================

Installation completed! 🎉

EOF
