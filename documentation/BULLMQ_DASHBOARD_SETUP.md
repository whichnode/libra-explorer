# BullMQ Dashboard (Bull Board) Setup Guide

## Overview

The 0L Explorer includes a Bull Board web interface for monitoring BullMQ queues in real-time. This dashboard provides visual insights into queue processing, job statuses, and helps diagnose issues with data ingestion pipelines.

## Location and Configuration

- **Directory**: `/pacakges/bull-board/`
- **Main Application**: `index.js`
- **Default Port**: `8006`
- **Docker Port**: `8080` (internal)

## Local Setup

### Method 1: Direct Node.js

1. **Navigate to the Bull Board directory**:
```bash
cd pacakges/bull-board
```

2. **Install dependencies**:
```bash
npm install
```

3. **Configure environment variables**:

Create a `.env` file in the `pacakges/bull-board` directory:

```bash
# Redis connection
REDIS_HOST=127.0.0.1
REDIS_PORT=6379

# Queue names to monitor (comma-separated)
QUEUE_NAMES=ol-version,ol-version-batch,ol-clickhouse-ingestor,ol-parquet-producer,expired-transactions,accounts,validators,community-wallets,wallet-subscription,stats,node-watcher

# Port for the dashboard (optional, defaults to 8006)
PORT=8006
```

4. **Start the dashboard**:
```bash
npm start
```

5. **Access the dashboard**:
```
http://localhost:8006
```

### Method 2: Docker

1. **Navigate to the Bull Board directory**:
```bash
cd pacakges/bull-board
```

2. **Build the Docker image**:
```bash
./build.sh
```

Or manually:
```bash
docker build \
  --file ./Dockerfile \
  --tag bull-board:local \
  .
```

3. **Run the container**:
```bash
docker run -d \
  --name bull-board \
  -p 8006:8080 \
  -e REDIS_HOST=host.docker.internal \
  -e REDIS_PORT=6379 \
  -e QUEUE_NAMES="ol-version,ol-version-batch,ol-clickhouse-ingestor,ol-parquet-producer,expired-transactions,accounts,validators,community-wallets,wallet-subscription,stats,node-watcher" \
  bull-board:local
```

**Note**: Use `host.docker.internal` on Mac/Windows. On Linux, use `--network host` or the actual host IP.

4. **Access the dashboard**:
```
http://localhost:8006
```

## Available Queues

The following BullMQ queues are available for monitoring:

### Core Processing Queues
- **`ol-version`** - Individual blockchain version processing
- **`ol-version-batch`** - Batch version processing for historical data
- **`ol-clickhouse-ingestor`** - ClickHouse data ingestion from Parquet files
- **`ol-parquet-producer`** - Parquet file generation for analytics

### Transaction and Account Queues
- **`expired-transactions`** - Cleanup of expired pending transactions
- **`accounts`** - Account data processing and updates
- **`validators`** - Validator information and statistics
- **`community-wallets`** - Community wallet tracking

### Monitoring and Subscription Queues
- **`wallet-subscription`** - Wallet subscription notifications
- **`stats`** - Network statistics aggregation
- **`node-watcher`** - Node health monitoring

## Dashboard Features

### Main Overview
- **Queue List**: All configured queues with job counts
- **Status Indicators**: Visual status for each queue
- **Job Counts**: Active, waiting, completed, failed, delayed, and paused jobs

### Queue Details
Click on any queue to see:
- **Jobs by Status**: Filtered views of jobs
- **Job Timeline**: Visual representation of job processing
- **Processing Rate**: Jobs processed per minute/hour
- **Error Rate**: Failed job statistics

### Job Management
- **View Job Data**: Inspect job payload and results
- **Error Messages**: See failure reasons and stack traces
- **Retry Failed Jobs**: Manually retry individual or bulk jobs
- **Clean Queue**: Remove old completed/failed jobs
- **Promote Delayed Jobs**: Force delayed jobs to process immediately
- **Pause/Resume Queue**: Control queue processing

### Individual Job View
- **Job ID and Status**
- **Creation and Processing Timestamps**
- **Attempt Count**: Number of processing attempts
- **Job Data**: Input parameters
- **Return Value**: Processing results
- **Error Details**: Failure information if applicable
- **Logs**: Processing logs (if configured)

## Troubleshooting Stale Data

When investigating stale transaction data, focus on these areas:

### 1. Check `ol-version` Queue
- **Failed Jobs**: Look for repeated failures
- **Stuck Jobs**: Check "active" jobs running for too long
- **Job Data**: Inspect version numbers being processed
- **Error Messages**: Common issues:
  - RPC timeout errors
  - Network connectivity issues
  - Invalid version numbers

### 2. Monitor `ol-version-batch` Queue
- **Batch Processing**: Ensures historical data completeness
- **Large Jobs**: May take longer to process
- **Memory Issues**: Check for out-of-memory errors

### 3. Verify `ol-clickhouse-ingestor` Queue
- **Ingestion Status**: Confirms data reaches ClickHouse
- **Parquet File Issues**: File format or corruption errors
- **Database Errors**: Connection or insertion failures

### 4. Review Queue Metrics
- **Processing Rate**: Jobs/minute should be consistent
- **Queue Depth**: Growing queues indicate processing issues
- **Failure Rate**: High failure rates need investigation

## Common Issues and Solutions

### Issue: Queue Shows No Jobs
**Solution**: Verify the worker is enabled in `ROLES` environment variable
```bash
ROLES=api,version-processor,clickhouse-ingestor-processor,...
```

### Issue: All Jobs Failing
**Possible Causes**:
- Redis connection issues
- RPC provider down
- Transformer binary missing
- Database connection problems

**Debug Steps**:
1. Check job error messages in dashboard
2. Verify Redis connectivity
3. Test RPC provider endpoint
4. Check application logs

### Issue: Jobs Stuck in "Active" State
**Solution**:
- Jobs may have timed out
- Click on the job to see details
- Use "Retry" to reprocess
- Check worker process health

### Issue: Growing "Delayed" Jobs
**Indicates**: Rate limiting or scheduled processing
- Check job details for delay reasons
- Use "Promote" to process immediately if needed

## Performance Optimization

### Queue Configuration
Monitor these metrics for optimization:
- **Concurrency**: Number of parallel jobs
- **Rate Limiting**: Jobs per time period
- **Retry Strategy**: Backoff configuration

### Redis Connection
Ensure Redis has sufficient:
- Memory for job storage
- Connection pool size
- Network bandwidth

### Worker Scaling
If queues are backing up:
1. Check CPU/memory usage of workers
2. Consider running multiple worker instances
3. Adjust job concurrency settings

## Integration with Monitoring

### Alerts to Set Up
Based on dashboard metrics, configure alerts for:
- Failed job count > threshold
- Queue depth > maximum
- Processing rate < minimum
- No jobs processed in X minutes

### Metrics to Track
- Jobs processed per minute
- Average processing time
- Failure rate percentage
- Queue depth trends

## Security Considerations

### Production Deployment
1. **Authentication**: Add authentication middleware
2. **Network Access**: Restrict to internal network or VPN
3. **Read-Only Access**: Consider read-only mode for production
4. **HTTPS**: Use reverse proxy with SSL

### Example Nginx Configuration
```nginx
server {
    listen 443 ssl;
    server_name bull-dashboard.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:8006;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;

        # Basic authentication
        auth_basic "Bull Board Dashboard";
        auth_basic_user_file /etc/nginx/.htpasswd;
    }
}
```

## Additional Resources

### Environment Variables Reference
- `REDIS_HOST`: Redis server hostname (default: "127.0.0.1")
- `REDIS_PORT`: Redis server port (default: 6379)
- `QUEUE_NAMES`: Comma-separated list of queue names to monitor
- `PORT`: HTTP port for dashboard (default: 8006)

### Related Documentation
- [Bull Board GitHub](https://github.com/felixmosh/bull-board)
- [BullMQ Documentation](https://docs.bullmq.io/)
- [Redis Administration](https://redis.io/docs/manual/admin/)

### Support Commands
```bash
# Check if Bull Board is running
curl http://localhost:8006/

# View Docker logs
docker logs bull-board

# Check Redis connectivity
redis-cli ping

# List all Bull queues in Redis
redis-cli --scan --pattern "bull:*"
```