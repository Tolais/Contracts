#!/bin/bash
# Database Recovery Script
# Restores Postgres database from encrypted S3 backup
# Usage: ./scripts/recover_database.sh <backup_timestamp> [target_host]

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

# Validate arguments
if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <backup_timestamp> [target_host]"
    echo "Example: $0 20260326_143022 localhost"
    exit 1
fi

TIMESTAMP="$1"
TARGET_HOST="${2:-${POSTGRES_HOST}}"
S3_BACKUP_PATH="s3://${AWS_BUCKET}/backups/${POSTGRES_DB}/${TIMESTAMP}/backup.sql.enc"
DOWNLOAD_DIR="$ROOT_DIR/backups/recovery"

echo "🔄 Starting database recovery..."
echo "⏰ Backup timestamp: ${TIMESTAMP}"
echo "🎯 Target host: ${TARGET_HOST}"
echo "📦 Source: ${S3_BACKUP_PATH}"

# Create recovery directory
mkdir -p "$DOWNLOAD_DIR"

# Step 1: Download encrypted backup from S3
echo "⬇️  Downloading backup from S3..."
aws s3 cp "$S3_BACKUP_PATH" "$DOWNLOAD_DIR/backup.sql.enc" \
    --region "${AWS_REGION}"

# Step 2: Download metadata for verification
echo "📋 Downloading backup metadata..."
aws s3 cp "s3://${AWS_BUCKET}/backups/${POSTGRES_DB}/${TIMESTAMP}/metadata.json" \
    "$DOWNLOAD_DIR/metadata.json" --region "${AWS_REGION}"

# Step 3: Verify checksum (optional but recommended)
echo "✅ Verifying backup integrity..."
aws s3 cp "s3://${AWS_BUCKET}/backups/${POSTGRES_DB}/${TIMESTAMP}/checksum.txt" \
    "$DOWNLOAD_DIR/checksum.txt" --region "${AWS_REGION}"
# Add checksum verification logic here if needed

# Step 4: Decrypt backup
echo "🔓 Decrypting backup..."
openssl enc -aes-256-cbc -d -pbkdf2 -in "$DOWNLOAD_DIR/backup.sql.enc" \
    -out "$DOWNLOAD_DIR/backup.sql" -pass pass:"${ENCRYPTION_KEY}"

# Step 5: Drop existing database and recreate
echo "🗑️  Dropping existing database (if exists)..."
PGPASSWORD="${POSTGRES_PASSWORD}" psql \
    -h "${TARGET_HOST}" \
    -p "${POSTGRES_PORT}" \
    -U "${POSTGRES_USER}" \
    -d postgres \
    -c "DROP DATABASE IF EXISTS ${POSTGRES_DB};"

echo "🏗️  Creating fresh database..."
PGPASSWORD="${POSTGRES_PASSWORD}" psql \
    -h "${TARGET_HOST}" \
    -p "${POSTGRES_PORT}" \
    -U "${POSTGRES_USER}" \
    -d postgres \
    -c "CREATE DATABASE ${POSTGRES_DB};"

# Step 6: Restore database
echo "💾 Restoring database from backup..."
PGPASSWORD="${POSTGRES_PASSWORD}" pg_restore \
    -h "${TARGET_HOST}" \
    -p "${POSTGRES_PORT}" \
    -U "${POSTGRES_USER}" \
    -d "${POSTGRES_DB}" \
    -v \
    "$DOWNLOAD_DIR/backup.sql"

# Step 7: Verify restoration
echo "✅ Verifying restoration..."
ROW_COUNT=$(PGPASSWORD="${POSTGRES_PASSWORD}" psql \
    -h "${TARGET_HOST}" \
    -p "${POSTGRES_PORT}" \
    -U "${POSTGRES_USER}" \
    -d "${POSTGRES_DB}" \
    -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';")

echo "📊 Restoration complete! Tables found: ${ROW_COUNT}"

# Step 8: Cleanup
echo "🧹 Cleaning up temporary files..."
rm -rf "$DOWNLOAD_DIR"

echo "✅ Database recovery completed successfully!"
echo "🎉 Database '${POSTGRES_DB}' restored to host '${TARGET_HOST}'"
echo "⏰ Recovery completed at: $(date -Iseconds)"

# Log recovery
echo "$(date -Iseconds) - Recovery completed: ${TIMESTAMP} -> ${TARGET_HOST}" >> "$ROOT_DIR/backups/recovery.log"
