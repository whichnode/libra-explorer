# Transaction Data Flow in 0L Explorer

## Overview
This document explains how transaction data flows from the blockchain to the latest transactions page, and identifies potential failure points that could cause stale data.

## Data Flow Pipeline

### 1. Frontend Query
**Location:** `web-app/src/modules/core/routes/Transactions/Transactions.tsx:11-26`

The transactions page makes a GraphQL query to fetch transaction data:
```graphql
query GetUserTransactions($limit: Int!, $offset: Int!) {
  userTransactions(limit: $limit, offset: $offset, order: "DESC") {
    size
    items {
      version
      sender
      moduleAddress
      moduleName
      functionName
      timestamp
      success
    }
  }
}
```

### 2. GraphQL Resolver
**Location:** `api/src/ol/user-transactions.resolver.ts:33-127`

- Directly queries the ClickHouse `user_transaction` table
- Returns paginated transaction data ordered by version (DESC by default)
- No caching layer - queries hit ClickHouse directly

### 3. Data Ingestion Pipeline

#### Version Processor
**Location:** `api/src/ol/ol-version.processor.ts`

The main ingestion flow:

1. **Fetch Latest Version** (runs every 5 seconds):
   - Calls `fetchLatestVersion()` (lines 93-96, 289-296)
   - Fetches latest ledger version from RPC provider
   - Creates processing jobs for the last 1,000 versions

2. **Process Individual Versions**:
   - Each version job fetches the transaction from blockchain API (lines 252-266)
   - Downloads transaction data from: `${RPC_PROVIDER_URL}/v1/transactions?start={version}&limit=1`

3. **Transform Data**:
   - Writes transaction JSON to temporary file (line 307)
   - Calls Rust transformer binary to convert to Parquet format (line 328)
   - Transformer location: `/usr/local/bin/transformer` (prod) or `./transformer/target/debug/transformer` (dev)

4. **Insert into ClickHouse**:
   - Inserts Parquet files directly into ClickHouse tables (line 331)
   - Marks version as ingested in `ingested_versions` table (lines 337-345)

#### Missing Versions Handler
**Location:** `api/src/ol/ol-version.processor.ts:384-412`

- Runs every 5 seconds to catch any missed versions
- Compares blockchain latest version with already ingested versions
- Creates jobs for any gaps in the sequence

## Potential Failure Points

### 1. RPC Provider Issues
**Impact:** No new transactions will be fetched

**Location:** `api/src/ol/ol-version.processor.ts:354-360`

**Symptoms:**
- `getLedgerVersion()` returns stale version numbers
- The RPC endpoint (`${RPC_PROVIDER_URL}/v1`) is down or unresponsive
- Network connectivity issues to the RPC provider

**Debug Commands:**
```bash
# Check current ledger version from RPC
curl ${RPC_PROVIDER_URL}/v1 | jq .ledger_version

# Check if endpoint is responsive
curl -w "\n%{http_code}\n" -o /dev/null -s ${RPC_PROVIDER_URL}/v1
```

### 2. Worker Role Not Enabled
**Impact:** Version processor never runs

**Required Environment Variable:**
```bash
ROLES="api,version-processor,..."
```

**Debug Commands:**
```bash
# Check if version-processor is in ROLES
echo $ROLES | grep version-processor

# Check running processes
ps aux | grep version-processor
```

### 3. Transformer Binary Failures
**Impact:** Transactions fetched but not transformed/ingested

**Location:** `api/src/ol/transformer.service.ts:155-194`

**Common Issues:**
- Binary not found at expected path
- Binary crashes during transformation
- Invalid JSON structure causes transformation failure
- Permission issues

**Debug Commands:**
```bash
# Check if transformer binary exists
ls -la /usr/local/bin/transformer  # Production
ls -la ./api/transformer/target/debug/transformer  # Development

# Check transformer logs for errors
grep "Transformer failed with code" /path/to/logs
grep "Transformer stderr:" /path/to/logs
```

### 4. Queue Processing Issues
**Impact:** Jobs created but not processed

**Configuration:**
- Jobs timeout after 1 minute (line 137)
- Failed jobs retry 15 times with 5-second delays (lines 273-276)
- Jobs removed after 1 hour when complete (line 280)

**Debug with BullMQ:**
```bash
# Connect to Redis
redis-cli

# Check queue status
KEYS bull:ol-version:*

# Check for failed jobs
LRANGE bull:ol-version:failed 0 -1

# Check for stuck jobs
ZRANGE bull:ol-version:stalled 0 -1
```

### 5. ClickHouse Ingestion Issues
**Impact:** Data transformed but not queryable

**Checks:**
```sql
-- Check latest ingested version
SELECT MAX(version) FROM user_transaction;

-- Check latest transaction timestamp
SELECT MAX(timestamp), FROM_UNIXTIME(MAX(timestamp)) FROM user_transaction;

-- Check ingested versions tracking
SELECT COUNT(*) FROM ingested_versions;
SELECT MAX(version) FROM ingested_versions;

-- Check for recent ingestions
SELECT COUNT(*)
FROM user_transaction
WHERE timestamp > (UNIX_TIMESTAMP() - 3600);
```

### 6. Duplicate Version Prevention
**Impact:** Versions marked as ingested but data missing

**Location:** `api/src/ol/ol-version.processor.ts:311-326`

The system checks `ingested_versions` before processing. If a version is marked as ingested but data is missing from `user_transaction`, it won't be re-processed.

**Fix:**
```sql
-- Find and remove incorrectly marked versions
DELETE FROM ingested_versions
WHERE version NOT IN (
  SELECT DISTINCT version FROM user_transaction
);
```

## Monitoring Checklist

### Real-time Monitoring
1. **RPC Health**
   - Monitor `${RPC_PROVIDER_URL}/v1` response times
   - Track ledger version progression

2. **Worker Health**
   - Ensure `version-processor` is in ROLES
   - Monitor worker process CPU/memory usage
   - Check BullMQ queue depths

3. **Database Health**
   - Monitor ClickHouse query performance
   - Track table sizes and growth rates
   - Monitor ingestion rates

### Key Metrics to Track
```sql
-- Ingestion lag (difference between blockchain and database)
WITH latest_blockchain AS (
  -- This would come from RPC call
  SELECT 1000000 as version
),
latest_db AS (
  SELECT MAX(version) as version FROM user_transaction
)
SELECT
  lb.version - ld.version as version_lag,
  NOW() - FROM_UNIXTIME(
    (SELECT MAX(timestamp) FROM user_transaction)
  ) as time_lag
FROM latest_blockchain lb, latest_db ld;

-- Ingestion rate (transactions per minute)
SELECT
  COUNT(*) as txn_count,
  FROM_UNIXTIME(MIN(timestamp)) as period_start,
  FROM_UNIXTIME(MAX(timestamp)) as period_end
FROM user_transaction
WHERE timestamp > (UNIX_TIMESTAMP() - 300);
```

## Recovery Procedures

### 1. Restart Stuck Workers
```bash
# Restart the API service (includes workers)
npm run start:dev  # Development
pm2 restart api    # Production with PM2
kubectl rollout restart deployment/api  # Kubernetes
```

### 2. Clear Failed Jobs
```bash
# Connect to Redis
redis-cli

# Clear failed jobs queue
DEL bull:ol-version:failed

# Clear completed jobs
DEL bull:ol-version:completed
```

### 3. Force Re-ingestion
```sql
-- Remove ingestion markers for a range
DELETE FROM ingested_versions
WHERE version BETWEEN :start_version AND :end_version;
```

Then restart the worker to trigger re-processing.

### 4. Manual Version Processing
```javascript
// Trigger specific version processing via API or console
await olVersionQueue.add('version', {
  version: '123456789'
}, {
  jobId: `__version__123456789`
});
```

## Common Scenarios

### Scenario 1: "No new transactions for several days"
**Likely Causes:**
1. RPC provider returning stale data
2. Worker not running
3. Transformer binary issues

**Investigation Steps:**
1. Check RPC provider ledger version
2. Verify worker is enabled in ROLES
3. Check logs for transformer errors
4. Query ClickHouse for latest data
5. Check BullMQ for failed jobs

### Scenario 2: "Transactions appear with delay"
**Likely Causes:**
1. Queue backlog
2. Slow RPC responses
3. ClickHouse ingestion delays

**Investigation Steps:**
1. Check queue depth in Redis
2. Monitor RPC response times
3. Check ClickHouse insert performance

## Environment Variables

Critical configuration for transaction processing:

```bash
# RPC endpoint for fetching blockchain data
RPC_PROVIDER_URL=https://rpc.provider.example.com

# Worker roles (must include version-processor)
ROLES=api,version-processor,clickhouse-ingestor-processor

# Database connections
CLICKHOUSE_HOST=127.0.0.1
CLICKHOUSE_DATABASE=olfyi
REDIS_HOST=127.0.0.1

# For batch processing (optional)
DATA_API_HOST=https://data.provider.example.com
```

## Contact Points

For issues with:
- **RPC Provider**: Check provider status page or contact provider support
- **ClickHouse**: Database administrator or DevOps team
- **Application Logs**: Check application logging system (CloudWatch, Datadog, etc.)
- **Queue Issues**: Redis/BullMQ monitoring dashboard