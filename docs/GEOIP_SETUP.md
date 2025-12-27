# MaxMind GeoIP2 Setup Guide

## Overview

Vision Node now supports **production-grade geographic detection** using MaxMind GeoIP2 databases, providing ~99.8% accuracy for country-level detection and city-level granularity.

## Features

- ✅ **High Accuracy**: ~99.8% country-level, ~90% city-level
- ✅ **Detailed Location**: Continent > Country > City format
- ✅ **IPv4 & IPv6 Support**: Full IP address coverage
- ✅ **Automatic Fallback**: Uses simple detection if database unavailable
- ✅ **Zero Configuration**: Works automatically when database is present
- ✅ **Performance**: Fast lookups with in-memory database caching

## Database Installation

### Option 1: GeoLite2 (Free, Recommended for Testing)

1. **Sign up for MaxMind GeoLite2**:
   - Visit: https://dev.maxmind.com/geoip/geolite2-free-geolocation-data
   - Create free account
   - Generate license key

2. **Download Database**:
   ```bash
   # Download GeoLite2-City (recommended - includes city data)
   curl -o GeoLite2-City.tar.gz \
     "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key=YOUR_LICENSE_KEY&suffix=tar.gz"
   
   # OR download GeoLite2-Country (smaller, country-level only)
   curl -o GeoLite2-Country.tar.gz \
     "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key=YOUR_LICENSE_KEY&suffix=tar.gz"
   ```

3. **Extract to Vision Node Directory**:
   ```bash
   # Extract archive
   tar -xzf GeoLite2-City.tar.gz
   
   # Move database file
   mkdir -p vision_data
   mv GeoLite2-City_*/GeoLite2-City.mmdb vision_data/
   ```

### Option 2: GeoIP2 Precision (Commercial, Higher Accuracy)

For production deployments requiring maximum accuracy:

1. Purchase GeoIP2 Precision City from MaxMind
2. Download the `.mmdb` database file
3. Place in `vision_data/GeoLite2-City.mmdb` or `vision_data/GeoLite2-Country.mmdb`

## File Locations

Vision Node checks for GeoIP2 databases in this order:

1. `vision_data/GeoLite2-City.mmdb` (highest priority)
2. `vision_data/GeoLite2-Country.mmdb`
3. `GeoLite2-City.mmdb` (current directory)
4. `GeoLite2-Country.mmdb` (current directory)

**Recommended**: Place database in `vision_data/` directory.

## Usage

### Automatic Operation

GeoIP2 detection works automatically when database is present:

```bash
# Start node with GeoIP2 database installed
./vision-node

# Check logs for GeoIP2 detection:
# [p2p::connection] Detected peer region (MaxMind): North America > United States > New York
```

### Region Format

**With GeoIP2 Database**:
- City database: `"North America > United States > New York"`
- Country database: `"Europe > Germany"`
- Private IPs: `"Private"`
- Local IPs: `"Local"`

**Without GeoIP2 (Fallback)**:
- Continent-level: `"North America"`, `"Europe"`, `"Asia"`, etc.
- Accuracy: ~80-90% continent detection
- Log note: "GeoIP2 database not available, using fallback detection"

### Verification

Check if GeoIP2 is working:

```bash
# Look for GeoIP2 success messages in logs
tail -f vision_data/logs/vision-node.log | grep "GeoIP2"

# Expected output:
# [p2p::connection] GeoIP2 City lookup successful
# [p2p::connection] Detected peer region (MaxMind): Europe > United Kingdom > London
```

## Performance

- **Lookup Speed**: ~10-50 microseconds per lookup (in-memory)
- **Memory Usage**: ~50-100 MB (City DB), ~10-20 MB (Country DB)
- **Database Updates**: Recommended monthly for accuracy

## Database Updates

### Manual Update

```bash
# Download latest database
curl -o GeoLite2-City.tar.gz \
  "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key=YOUR_LICENSE_KEY&suffix=tar.gz"

# Extract and replace
tar -xzf GeoLite2-City.tar.gz
mv GeoLite2-City_*/GeoLite2-City.mmdb vision_data/GeoLite2-City.mmdb

# Restart node to use new database
systemctl restart vision-node
```

### Automatic Update Script (Optional)

Create `update-geoip.sh`:

```bash
#!/bin/bash
LICENSE_KEY="YOUR_LICENSE_KEY"
DB_DIR="vision_data"

echo "Updating GeoIP2 database..."
curl -o GeoLite2-City.tar.gz \
  "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key=${LICENSE_KEY}&suffix=tar.gz"

tar -xzf GeoLite2-City.tar.gz
mv GeoLite2-City_*/GeoLite2-City.mmdb ${DB_DIR}/
rm -rf GeoLite2-City_* GeoLite2-City.tar.gz

echo "Database updated successfully"
```

Schedule with cron (monthly):
```bash
0 0 1 * * /path/to/update-geoip.sh
```

## Troubleshooting

### Database Not Found

**Symptom**: Log shows "GeoIP2 database not available, using fallback detection"

**Solution**:
1. Verify database file exists: `ls -la vision_data/GeoLite2-*.mmdb`
2. Check file permissions: `chmod 644 vision_data/GeoLite2-*.mmdb`
3. Verify database is not corrupted: Use `mmdbinspect` tool

### Database Corruption

**Symptom**: "Failed to open GeoIP2 database" error

**Solution**:
1. Re-download database from MaxMind
2. Verify download integrity
3. Clear any partial downloads

### No Region Detected

**Symptom**: All peers show "Unknown" region

**Solution**:
1. Verify peers have public IP addresses (not RFC 1918 private IPs)
2. Check database covers IP ranges (GeoLite2 covers ~99.8% of IPs)
3. Review logs for specific lookup errors

## License Requirements

### GeoLite2 (Free)

- ✅ Free for internal use
- ✅ Requires attribution in public-facing applications
- ✅ Database updates available weekly
- ⚠️ Slightly lower accuracy than commercial version

### GeoIP2 Precision (Commercial)

- 💰 Paid license required
- ✅ Highest accuracy (99.8%+ country, 90%+ city)
- ✅ Daily updates available
- ✅ Commercial use allowed
- ✅ No attribution required

## Attribution (for GeoLite2)

If using GeoLite2 in public-facing logs/dashboards, include:

> This product includes GeoLite2 data created by MaxMind, available from https://www.maxmind.com

## API Reference

### Environment Variables

No additional environment variables needed. Database detection is automatic.

### Log Format

```
[p2p::connection] Detected peer region (MaxMind): <Continent> > <Country> > <City>
[p2p::connection] method = "GeoIP2"
```

### Fallback Mode

```
[p2p::connection] Detected peer region (fallback): <Continent>
[p2p::connection] method = "fallback"
```

## Best Practices

1. **Use City Database**: More detailed than Country database
2. **Update Monthly**: MaxMind updates GeoLite2 monthly
3. **Monitor Logs**: Check for "Failed to open GeoIP2 database" warnings
4. **Backup Database**: Keep backup copy during updates
5. **Production**: Use GeoIP2 Precision for mission-critical applications

## Support

- **MaxMind Documentation**: https://dev.maxmind.com/geoip/docs
- **Database Downloads**: https://dev.maxmind.com/geoip/geolite2-free-geolocation-data
- **Issue Tracker**: Report Vision Node issues on GitHub

## Example Output

### With GeoIP2:
```
[p2p::connection] Detected peer region (MaxMind): North America > United States > New York
[p2p::connection] Detected peer region (MaxMind): Europe > United Kingdom > London
[p2p::connection] Detected peer region (MaxMind): Asia > Japan > Tokyo
```

### Without GeoIP2 (Fallback):
```
[p2p::connection] GeoIP2 database not available, using fallback detection
[p2p::connection] Detected peer region (fallback): North America
[p2p::connection] Detected peer region (fallback): Europe
[p2p::connection] Detected peer region (fallback): Asia
```
