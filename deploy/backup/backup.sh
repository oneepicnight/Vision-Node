#!/bin/bash
# Vision Node Backup Script
# Performs snapshot backup with S3 upload and retention management

set -e

# Configuration from environment
BACKUP_INTERVAL=${BACKUP_INTERVAL:-86400}  # 24 hours
BACKUP_RETENTION_DAYS=${BACKUP_RETENTION_DAYS:-7}
DATA_DIR=${DATA_DIR:-/data}
BACKUP_DIR=${BACKUP_TARGET:-/backups}
S3_BUCKET=${S3_BUCKET:-}
S3_REGION=${S3_REGION:-us-east-1}

echo "🔄 Vision Node Backup Service Starting"
echo "========================================="
echo "  Data Directory: $DATA_DIR"
echo "  Backup Directory: $BACKUP_DIR"
echo "  Backup Interval: ${BACKUP_INTERVAL}s ($(($BACKUP_INTERVAL / 3600))h)"
echo "  Retention: $BACKUP_RETENTION_DAYS days"
if [ -n "$S3_BUCKET" ]; then
    echo "  S3 Bucket: s3://$S3_BUCKET"
fi
echo ""

# Function to create backup
create_backup() {
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_name="vision-node-backup-${timestamp}.tar.gz"
    local backup_path="${BACKUP_DIR}/${backup_name}"
    
    echo "📦 Creating backup: $backup_name"
    
    # Create compressed backup
    tar -czf "$backup_path" -C "$DATA_DIR" . 2>/dev/null || {
        echo "❌ Backup creation failed"
        return 1
    }
    
    local backup_size=$(du -h "$backup_path" | cut -f1)
    echo "✅ Backup created: $backup_size"
    
    # Upload to S3 if configured
    if [ -n "$S3_BUCKET" ]; then
        echo "☁️  Uploading to S3..."
        if command -v aws &> /dev/null; then
            aws s3 cp "$backup_path" "s3://${S3_BUCKET}/backups/${backup_name}" \
                --region "$S3_REGION" \
                --storage-class STANDARD_IA || {
                echo "⚠️  S3 upload failed"
            }
            echo "✅ Uploaded to S3"
        else
            echo "⚠️  AWS CLI not found, skipping S3 upload"
        fi
    fi
    
    echo ""
}

# Function to clean old backups
cleanup_old_backups() {
    echo "🧹 Cleaning up old backups (retention: $BACKUP_RETENTION_DAYS days)"
    
    # Clean local backups
    find "$BACKUP_DIR" -name "vision-node-backup-*.tar.gz" -type f -mtime +$BACKUP_RETENTION_DAYS -delete 2>/dev/null || true
    
    local remaining=$(find "$BACKUP_DIR" -name "vision-node-backup-*.tar.gz" -type f | wc -l)
    echo "   Local backups remaining: $remaining"
    
    # Clean S3 backups if configured
    if [ -n "$S3_BUCKET" ] && command -v aws &> /dev/null; then
        local cutoff_date=$(date -d "$BACKUP_RETENTION_DAYS days ago" +%Y%m%d 2>/dev/null || \
                           date -v-${BACKUP_RETENTION_DAYS}d +%Y%m%d 2>/dev/null)
        
        aws s3 ls "s3://${S3_BUCKET}/backups/" --region "$S3_REGION" 2>/dev/null | \
        while read -r line; do
            local s3_file=$(echo "$line" | awk '{print $4}')
            if [ -n "$s3_file" ]; then
                local file_date=$(echo "$s3_file" | grep -oP '\d{8}' | head -1)
                if [ -n "$file_date" ] && [ "$file_date" -lt "$cutoff_date" ]; then
                    echo "   Deleting old S3 backup: $s3_file"
                    aws s3 rm "s3://${S3_BUCKET}/backups/${s3_file}" --region "$S3_REGION" 2>/dev/null || true
                fi
            fi
        done
    fi
    
    echo ""
}

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

# Main backup loop
echo "🚀 Backup service running (interval: ${BACKUP_INTERVAL}s)"
echo ""

while true; do
    echo "========================================="
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Starting backup cycle"
    echo "========================================="
    
    create_backup
    cleanup_old_backups
    
    echo "💤 Sleeping for ${BACKUP_INTERVAL}s until next backup..."
    echo ""
    sleep "$BACKUP_INTERVAL"
done
