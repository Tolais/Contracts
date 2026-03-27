# Disaster Recovery & Backup Procedures

## Overview

This document outlines the disaster recovery (DR) procedures for the Stellar Protocol database infrastructure. All backups are encrypted and stored in AWS S3 with automated daily Point-in-Time recovery capabilities.

## Architecture

```
┌─────────────────┐      ┌──────────────┐      ┌─────────────┐
│  PostgreSQL DB  │─────►│ Backup Script│─────►│ Encrypted   │
│  (Production)   │ pg_dump│ (AES-256)    │ S3    │ S3 Bucket   │
└─────────────────┘      └──────────────┘      └─────────────┘
                                ▲
                                │
                         ┌──────┴──────┐
                         │Recovery Script│
                         └──────────────┘
```

## Backup Configuration

### Prerequisites

1. **AWS CLI** configured with appropriate credentials
2. **PostgreSQL client tools** (`psql`, `pg_dump`, `pg_restore`)
3. **OpenSSL** for encryption/decryption
4. **Configuration file**: Copy `.env.backup.example` to `.env.backup`

### Setup Steps

```bash
# 1. Generate encryption key
openssl rand -base64 32 > encryption_key.txt

# 2. Configure environment
cp .env.backup.example .env.backup
# Edit .env.backup with your credentials

# 3. Make scripts executable
chmod +x scripts/*.sh

# 4. Test backup
./scripts/backup_database.sh
```

## Automated Daily Backups

### Cron Schedule

Add to crontab for automated daily backups at 2 AM UTC:

```cron
0 2 * * * /path/to/scripts/backup_database.sh >> /var/log/backup.log 2>&1
```

### Backup Structure

S3 bucket structure:
```
s3://your-bucket/
├── backups/
│   └── stellar_protocol/
│       ├── 20260326_143022/
│       │   ├── backup.sql.enc
│       │   ├── metadata.json
│       │   └── checksum.txt
│       ├── 20260327_020000/
│       │   └── ...
```

## Recovery Procedures

### Standard Recovery

To recover from a specific backup:

```bash
# List available backups
aws s3 ls s3://your-bucket/backups/stellar_protocol/

# Recover using timestamp
./scripts/recover_database.sh 20260326_143022
```

### Recovery to New Server

```bash
# Recover to different host
./scripts/recover_database.sh 20260326_143022 new-db.example.com

# Or set environment variables
export POSTGRES_HOST=new-db.example.com
export POSTGRES_PORT=5432
./scripts/recover_database.sh 20260326_143022
```

## Fire Drill Testing

### Quarterly DR Tests

Run fire drills quarterly to verify RTO < 30 minutes:

```bash
./scripts/fire_drill.sh
```

### What the Fire Drill Tests

1. ✅ Locates most recent backup from S3
2. ✅ Downloads and decrypts backup
3. ✅ Restores to isolated test server
4. ✅ Verifies data integrity
5. ✅ Measures total recovery time
6. ✅ Generates compliance report
7. ✅ Cleans up test environment

### Fire Drill Report

After completion, a detailed report is generated at:
`backups/fire_drill_report_<timestamp>.md`

## Security Considerations

### Encryption

- All backups encrypted with AES-256-CBC
- Keys stored separately from backups
- PBKDF2 key derivation for added security

### Access Control

- S3 bucket policies restrict access to specific IAM roles
- Database credentials rotated monthly
- Backup logs audited quarterly

### Compliance

- Backups retained for 90 days minimum
- Fire drills documented and archived
- RTO/RPO metrics tracked over time

## Troubleshooting

### Common Issues

**Backup fails with permission error:**
```bash
# Verify AWS credentials
aws sts get-caller-identity

# Check S3 bucket policy
aws s3api get-bucket-policy --bucket your-bucket
```

**Recovery fails with connection error:**
```bash
# Test database connectivity
psql -h localhost -p 5432 -U postgres -c "SELECT 1"

# Check PostgreSQL is running
pg_isready -h localhost -p 5432
```

**Decryption fails:**
```bash
# Verify encryption key format
cat .env.backup | grep ENCRYPTION_KEY

# Regenerate if needed (WARNING: old backups will be inaccessible)
openssl rand -base64 32
```

## Monitoring & Alerting

### Health Checks

Monitor backup success with:

```bash
# Check last successful backup
tail -n 1 backups/backup.log

# Verify S3 has recent backups
aws s3 ls s3://your-bucket/backups/stellar_protocol/ | tail -n 1
```

### Alerting Rules

Set up alerts for:
- ❌ No backup in last 24 hours
- ❌ Fire drill RTO > 30 minutes
- ❌ S3 bucket storage threshold exceeded
- ⚠️ Backup duration exceeds 1 hour

## Contact & Escalation

For issues with backup/recovery:
1. Check this documentation
2. Review recent fire drill reports
3. Contact DevOps team
4. Escalate to CTO if critical

---

**Last Updated**: 2026-03-26  
**Last Fire Drill**: TBD  
**RTO Target**: < 30 minutes  
**RPO Target**: < 24 hours
