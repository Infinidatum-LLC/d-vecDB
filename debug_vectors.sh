#!/bin/bash

# Debug script for d-vecDB vector loading issue
# Usage: ./debug_vectors.sh /path/to/data/dir collection_name

DATA_DIR="${1:-/root/embedding-project/dvecdb-data}"
COLLECTION="${2:-incidents}"

echo "==================================="
echo "d-vecDB Vector Loading Debug Script"
echo "==================================="
echo ""

# Check 1: File existence and size
echo "1. Checking files..."
VECTORS_BIN="$DATA_DIR/$COLLECTION/vectors.bin"
METADATA_JSON="$DATA_DIR/$COLLECTION/metadata.json"

if [ -f "$VECTORS_BIN" ]; then
    SIZE=$(stat -f%z "$VECTORS_BIN" 2>/dev/null || stat -c%s "$VECTORS_BIN" 2>/dev/null)
    echo "✅ vectors.bin exists: $(ls -lh $VECTORS_BIN | awk '{print $5}')"
    echo "   Raw size: $SIZE bytes"
else
    echo "❌ vectors.bin NOT FOUND at: $VECTORS_BIN"
    exit 1
fi

if [ -f "$METADATA_JSON" ]; then
    echo "✅ metadata.json exists"
    cat "$METADATA_JSON" | head -20
else
    echo "❌ metadata.json NOT FOUND at: $METADATA_JSON"
fi

echo ""

# Check 2: File format - read first few bytes
echo "2. Checking file format..."
echo "   First 16 bytes (hex):"
xxd -l 16 "$VECTORS_BIN" 2>/dev/null || hexdump -n 16 -C "$VECTORS_BIN" 2>/dev/null
echo ""

# Check 3: Try to read length prefix
echo "3. Reading length prefix (first 4 bytes as u32 little-endian)..."
if command -v python3 &> /dev/null; then
    python3 << 'EOF'
import sys
import struct

try:
    with open(sys.argv[1], 'rb') as f:
        length_bytes = f.read(4)
        if len(length_bytes) == 4:
            length = struct.unpack('<I', length_bytes)[0]
            print(f"   Length prefix: {length} bytes")
            if length > 1000000:
                print(f"   ⚠️  WARNING: Length is very large ({length} bytes)")
                print(f"   This might indicate wrong format or corrupted file")
            elif length == 0:
                print(f"   ❌ ERROR: Length is 0, file might be empty or wrong format")
            else:
                print(f"   ✅ Length looks reasonable")
                # Try to read that much data
                data = f.read(length)
                print(f"   Successfully read {len(data)} bytes of data")
        else:
            print("   ❌ ERROR: Could not read 4 bytes for length prefix")
except Exception as e:
    print(f"   ❌ ERROR: {e}")
EOF
else
    echo "   ⚠️  Python3 not available, skipping length check"
fi

echo ""

# Check 4: Server logs check
echo "4. Checking recent server logs for vector loading..."
if [ -f "/var/log/d-vecdb/server.log" ]; then
    echo "   Checking /var/log/d-vecdb/server.log..."
    grep -E "(Loaded|Loading|Successfully rebuilt|Failed to|ERROR)" /var/log/d-vecdb/server.log | tail -20
elif command -v journalctl &> /dev/null; then
    echo "   Checking journalctl..."
    journalctl -u d-vecdb -n 50 | grep -E "(Loaded|Loading|Successfully rebuilt|Failed to|ERROR)"
else
    echo "   ⚠️  No logs found. Start server with: ./vectordb-server ... 2>&1 | tee server.log"
fi

echo ""
echo "==================================="
echo "Debug Summary"
echo "==================================="
echo ""
echo "Next steps:"
echo "1. If length prefix looks wrong, the file format may be incompatible"
echo "2. If 'Loaded 0 vectors' in logs, check for deserialization errors"
echo "3. If no loading messages in logs, the server might not have the fix"
echo ""
echo "To get detailed logs, run:"
echo "  cd /root/d-vecDB"
echo "  RUST_LOG=debug ./target/release/vectordb-server \\"
echo "    --host 0.0.0.0 --port 8080 \\"
echo "    --data-dir $DATA_DIR 2>&1 | tee debug.log"
echo ""
