#!/bin/bash

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install Rust
install_rust() {
    echo "Installing Rust and Cargo..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "Rust installation complete.\n"
}

# Check if Rust is installed
if ! command_exists rustc; then
    install_rust
else
    echo -e "Rust is already installed.\n"
fi

# Ensure Cargo is available
if ! command_exists cargo; then
    echo -e "\nCargo is not found. Please check your Rust installation."
    exit 1
fi

# List of Cargo tools to check and install
CARGO_TOOLS=(
    "cargo-tarpaulin" 
    "cargo-generate" 
    "cargo-concordium"
)

# Function to install Cargo tools if not installed
install_cargo_tools() {
    for tool in "${CARGO_TOOLS[@]}"; do
        if ! cargo install --list | grep -q "$tool"; then
            echo "Installing $tool..."
            cargo install "$tool"
        else
            echo "$tool is already installed."
        fi
    done
}

# Install missing Cargo tools
install_cargo_tools

echo ""

read -p "Do you want to install Concordium-Client? (y/n): " choice
choice=$(echo "$choice" | tr '[:upper:]' '[:lower:]')

echo ""

# Concordium-Client CLI installation (Optional)
if [[ "$choice" == "y" || "$choice" == "yes" ]]; then
    curl -L -o concordium-client https://distribution.concordium.software/tools/linux/concordium-client_8.0.0-5?
    chmod +x concordium-client
    sudo mv concordium-client /usr/local/bin/concordium-client

    echo ""

    read -p "Provide wallet keys <Wallet.export> and name to configure concordium-client: " keys name

    if [[ -z "$keys" || -z "$name" ]]; then
        echo -e "\nWallet key and name are not provided, skipping configurations."
    else 
        concordium-client config account import $keys --name $name.json
    fi
fi

echo -e "\nSetup completed. Installed concordium-client, Rust, Cargo, and cargo-tools."