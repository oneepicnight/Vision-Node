# Backup Service Container
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    tar \
    gzip \
    awscli \
    && rm -rf /var/lib/apt/lists/*

# Create backup user
RUN useradd -m -u 1000 -s /bin/bash backup

# Create directories
RUN mkdir -p /data /backups && \
    chown -R backup:backup /backups

# Copy backup script
COPY backup.sh /usr/local/bin/backup.sh
RUN chmod +x /usr/local/bin/backup.sh

# Switch to backup user
USER backup

WORKDIR /backups

# Run backup service
CMD ["/usr/local/bin/backup.sh"]
