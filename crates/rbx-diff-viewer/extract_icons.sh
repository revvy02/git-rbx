#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
JSON_FILE="$SCRIPT_DIR/class_icons.json"
OUTPUT_DIR="$SCRIPT_DIR/images"

# Detect Roblox content path
if [[ "$OSTYPE" == "darwin"* ]]; then
    CONTENT_PATH="/Applications/RobloxStudio.app/Contents/Resources/content"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    # Find the latest Roblox version folder
    ROBLOX_BASE="$LOCALAPPDATA/Roblox/Versions"
    CONTENT_PATH=$(find "$ROBLOX_BASE" -maxdepth 1 -name "version-*" -type d | sort -r | head -1)/content
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

if [[ ! -d "$CONTENT_PATH" ]]; then
    echo "Error: Roblox content folder not found at $CONTENT_PATH"
    echo "Please ensure Roblox Studio is installed."
    exit 1
fi

echo "[extract_icons] Using content path: $CONTENT_PATH"
echo "[extract_icons] Output directory: $OUTPUT_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Extract unique rbxasset:// URLs and copy files
URLS=$(grep -o '"rbxasset://[^"]*"' "$JSON_FILE" | sort -u | tr -d '"')
TOTAL=$(echo "$URLS" | wc -l | tr -d ' ')
COPIED=0
MISSING=0

echo "[extract_icons] Found $TOTAL unique icon paths"

while IFS= read -r url; do
    # Convert rbxasset:// URL to local path
    relative_path="${url#rbxasset://}"
    source_file="$CONTENT_PATH/$relative_path"

    # Flatten to single directory (just filename)
    filename=$(basename "$relative_path")
    dest_file="$OUTPUT_DIR/$filename"

    if [[ -f "$source_file" ]]; then
        cp "$source_file" "$dest_file"
        ((COPIED++))
    else
        echo "[extract_icons] Missing: $relative_path"
        ((MISSING++))
    fi
done <<< "$URLS"

echo "[extract_icons] Complete: $COPIED copied, $MISSING missing"
echo "[extract_icons] Icons saved to: $OUTPUT_DIR"
