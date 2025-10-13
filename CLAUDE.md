# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The 0L Network Explorer is a full-stack blockchain explorer application with:
- **api/**: NestJS GraphQL backend with ClickHouse analytics
- **web-app/**: React TypeScript frontend with Vite
- **api/transformer/**: Rust binary for blockchain data transformation
- **infra/**: Kubernetes deployment configurations
- **ol-fyi-local-infra/**: Docker Compose for local development

## Common Development Commands

### API (Backend)
```bash
cd api
npm install
npm run start:dev       # Development with hot reload (port 3000)
npm run build          # Production build
npm run test           # Run Jest unit tests
npm run test:watch     # Jest watch mode
npm run test:cov       # Coverage report
npm run test:e2e       # End-to-end tests
npm run lint           # ESLint with auto-fix
npm run format         # Prettier formatting
npx prisma generate    # Regenerate Prisma client after schema changes
npx prisma db push     # Push schema changes to database
```

### Web App (Frontend)
```bash
cd web-app
npm install
npm run dev            # Vite dev server (port 5173)
npm run build          # TypeScript + Vite production build
npm run lint           # ESLint
npm run preview        # Preview production build
npm run prettier-fix   # Auto-format code
```

### Transformer (Rust)
```bash
cd api/transformer
cargo build            # Build the transformer binary
```

## Architecture Overview

### Backend Architecture (NestJS)

**Module Structure** (`api/src/`):
- `app/app.module.ts`: Root module with dependency configuration
- `ol/`: Core blockchain data handling
  - `accounts/`: Account processing and resolution
  - `validators/`: Validator information and statistics
  - `transactions/`: Transaction factory pattern implementation
  - `movements/`: Token movement tracking
  - `community-wallets/`: Community wallet management
- `stats/`: Analytics and metrics processing
- `node-watcher/`: Network monitoring
- `clickhouse/`: ClickHouse database integration
- `multi-sig/`: Multi-signature wallet operations
- `redis/`, `nats/`, `s3/`, `firebase/`: Service integrations

**Role-Based Worker System**:
Workers are configured via the `ROLES` environment variable:
- `version-batch-processor`: Batch blockchain version processing
- `parquet-producer-processor`: Parquet file generation for analytics
- `version-processor`: Individual version processing
- `clickhouse-ingestor-processor`: Data ingestion to ClickHouse
- `expired-transactions-processor`: Clean up expired transactions
- `accounts-processor`: Account data processing

**GraphQL Patterns**:
- Resolvers use `@Resolver()` decorator with model types
- Field resolvers use `@ResolveField()` for nested data
- Pagination uses cursor-based approach with `PaginatedX` types
- Custom scalars for Buffer and BigNumber types

**Transaction Processing**:
Factory pattern in `api/src/ol/transactions/`:
- `TransactionsFactory.ts`: Creates transaction handlers
- `TransactionsService.ts`: Core business logic
- `TransactionsRepository.ts`: Database operations
- `OnChainTransactionsRepository.ts`: Blockchain data access

### Frontend Architecture (React)

**Module Structure** (`web-app/src/modules/`):
- `core/`: Application foundation
  - `router.tsx`: React Router v6 configuration
  - `apollo-client.ts`: GraphQL client setup with WebSocket subscriptions
  - `routes/`: Page components (Account, Stats, Validators, Blocks, etc.)
- `ui/`: Reusable UI components
- `interface/`: TypeScript interfaces and types
- `aptos/`: Wallet integration components
- `ol/`, `movements/`: Domain-specific modules

**Routing Structure**:
- `/accounts/:address`: Account details with nested routes (movements, transactions)
- `/transactions/:version`: Transaction details
- `/blocks/:blockHeight`: Block exploration
- `/validators`: Validator list and details
- `/stats`: Network statistics dashboard
- `/community-wallets`: Community wallet tracking

**State Management**:
- Apollo Client for GraphQL state and caching
- Real-time updates via GraphQL subscriptions
- Styled Components + Tailwind CSS for styling

### Data Flow
1. Blockchain data ingested via role-based processors
2. Rust transformer processes raw data (`api/transformer/`)
3. Processed data stored in ClickHouse (analytics) and PostgreSQL (app data)
4. GraphQL resolvers query both databases
5. React frontend consumes GraphQL API with real-time subscriptions via WebSocket

## Local Development Setup

### Prerequisites
- Docker and Docker Compose
- Node.js 22+
- Rust and Cargo
- PostgreSQL client tools

### Setup Steps

1. **Start infrastructure**:
   ```bash
   cd ol-fyi-local-infra
   docker compose up -d
   ```

2. **Run ClickHouse migrations**:
   ```bash
   docker compose exec -T clickhouse clickhouse-client --database=olfyi -n < ../api/tables_local.sql
   ```

3. **Build transformer**:
   ```bash
   cd api/transformer
   cargo build
   ```

4. **Setup API**:
   ```bash
   cd api
   npm install
   cp .env.example .env
   npx prisma generate
   npx prisma db push
   npm run start:dev
   ```

5. **Setup frontend**:
   ```bash
   cd web-app
   npm install
   npm run dev
   ```

## Environment Configuration

Key environment variables in `api/.env`:
- `DATABASE_URL`: PostgreSQL connection for Prisma
- `CLICKHOUSE_HOST`, `CLICKHOUSE_DATABASE`: ClickHouse connection
- `REDIS_HOST`: Redis for BullMQ queues
- `NATS_SERVERS`: NATS messaging system
- `RPC_PROVIDER_URL`: 0L blockchain RPC endpoint
- `DATA_API_HOST`: Blockchain data API
- `ROLES`: Comma-separated list of worker roles to enable
- `S3_*`: Object storage configuration for Parquet files

## Testing Strategy

### API Testing
- **Test Files**: `*.spec.ts` files co-located with source code
- **E2E Tests**: Separate `/test/` directory
- **Configuration**: Jest with ESM support via ts-jest
- **Run Tests**: `npm run test` (unit), `npm run test:e2e` (integration)

### Code Quality
- **Prettier**: Line width 100, single quotes, trailing commas
- **ESLint**: TypeScript configuration with auto-fix
- **Pre-commit**: Run `npm run lint` and `npm run prettier-fix`

## Key Architectural Patterns

1. **Dependency Injection**: NestJS DI for service management
2. **Factory Pattern**: Transaction processing with pluggable handlers
3. **Repository Pattern**: Clean data access layer abstractions
4. **Module Federation**: Feature-based module organization
5. **Real-time Updates**: GraphQL subscriptions over WebSocket
6. **Multi-database**: PostgreSQL (app), ClickHouse (analytics)
7. **Background Processing**: BullMQ queues with Redis
8. **Role-based Workers**: Configurable processing roles

## Database Schema

### PostgreSQL (Prisma)
- Application configuration
- User preferences
- Wallet subscriptions
- Multi-sig wallet data

### ClickHouse
- Blockchain transactions
- Account balances
- Validator statistics
- Network metrics
- Time-series analytics

## Deployment

### Docker Build
- **API**: Multi-stage build with Rust transformer and Node.js
- **Frontend**: Vite build with Nginx serving
- **Infrastructure**: Kubernetes manifests in `/infra/`

### Production Considerations
- Enable specific worker roles per deployment
- Configure appropriate database connections
- Set up S3 for Parquet file storage
- Configure Firebase for push notifications