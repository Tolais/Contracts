#!/bin/bash
# Point-in-Time Database Backup Script
# Backs up Postgres database to encrypted S3 bucket
# Usage: ./scripts/backup_database.sh [environment]

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="$ROOT_DIR/.env.backup"

# Load configuration
if [[ -f "$CONFIG_FILE" ]]; then
    source "$CONFIG_FILE"
else
    echo "Error: Configuration file $CONFIG_FILE not found"
    exit 1
fi

# Environment variables (set in .env.backup)
# POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD
# AWS_BUCKET, AWS_REGION, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
# ENCRYPTION_KEY (base64 encoded AES-256 key)

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_FILE="$ROOT_DIR/backups/${POSTGRES_DB}_${TIMESTAMP}.sql"
ENCRYPTED_FILE="${BACKUP_FILE}.enc"
S3_PATH="s3://${AWS_BUCKET}/backups/${POSTGRES_DB}/${TIMESTAMP}/"

# Create backups directory
mkdir -p "$ROOT_DIR/backups"

echo "🔄 Starting Point-in-Time backup for database: ${POSTGRES_DB}"
echo "📁 Backup location: ${S3_PATH}"

# Step 1: Create database dump
echo "💾 Creating database dump..."
PGPASSWORD="${POSTGRES_PASSWORD}" pg_dump \
    -h "${POSTGRES_HOST}" \
    -p "${POSTGRES_PORT}" \
    -U "${POSTGRES_USER}" \
    -d "${POSTGRES_DB}" \
    -F c \
    -b \
    -v \
    -f "$BACKUP_FILE"

# Step 2: Encrypt backup using OpenSSL (AES-256-CBC)
echo "🔒 Encrypting backup..."
openssl enc -aes-256-cbc -salt -pbkdf2 -in "$BACKUP_FILE" -out "$ENCRYPTED_FILE" -pass pass:"${ENCRYPTION_KEY}"

# Step 3: Upload to S3
echo "☁️  Uploading to S3..."
aws s3 cp "$ENCRYPTED_FILE" "${S3_PATH}backup.sql.enc" \
    --region "${AWS_REGION}" \
    --storage-class STANDARD_IA

# Step 4: Upload encryption metadata
echo "📝 Creating backup metadata..."
cat > "$ROOT_DIR/backups/metadata_${TIMESTAMP}.json" <<EOF
{
    "timestamp": "${TIMESTAMP}",
    "database": "${POSTGRES_DB}",
    "host": "${POSTGRES_HOST}",
    "encrypted_file": "${ENCRYPTED_FILE}",
    "s3_path": "${S3_PATH}",
    "encryption_algorithm": "AES-256-CBC",
    "backup_size_bytes": $(stat -f%z "$ENCRYPTED_FILE" 2>/dev/null || stat -c%s "$ENCRYPTED_FILE" 2>/dev/null || echo "0")
}
EOF

aws s3 cp "$ROOT_DIR/backups/metadata_${TIMESTAMP}.json" "${S3_PATH}metadata.json" \
    --region "${AWS_REGION}" \
    --content-type "application/json"

# Step 5: Cleanup local files
echo "🧹 Cleaning up local files..."
rm -f "$BACKUP_FILE" "$ENCRYPTED_FILE"

# Step 6: Create checksum file for verification
CHECKSUM_FILE="$ROOT_DIR/backups/checksum_${TIMESTAMP}.txt"
aws s3 ls "${S3_PATH}backup.sql.enc" --region "${AWS_REGION}" | awk '{print $1, $2, $3, $4}' > "$CHECKSUM_FILE"
aws s3 cp "$CHECKSUM_FILE" "${S3_PATH}checksum.txt" --region "${AWS_REGION}"
rm -f "$CHECKSUM_FILE"

echo "✅ Backup completed successfully!"
echo "📊 Backup saved to: ${S3_PATH}"
echo "⏰ Timestamp: ${TIMESTAMP}"

# Log backup
echo "$(date -Iseconds) - Backup completed: ${S3_PATH}" >> "$ROOT_DIR/backups/backup.log"
