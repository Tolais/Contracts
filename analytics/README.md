# Analytics Backend - Revenue Prediction System

## Overview

This analytics backend provides revenue prediction algorithms that analyze creator earnings based on active streams, historical churn rates, and growth trends. It serves data for a "Projected Revenue" chart showing 30, 60, and 90-day forecasts.

## Features

### 🎯 Revenue Prediction Algorithm

- **Monte Carlo Simulation**: Uses 1000+ iterations to model future revenue scenarios
- **Churn Analysis**: Calculates cancellation rates from historical stream data
- **Growth Trend Detection**: Applies linear regression to identify revenue trajectories
- **Volatility Modeling**: Incorporates standard deviation for risk assessment
- **Confidence Intervals**: Provides 95% confidence bounds on all predictions

### 📊 Prediction Factors

The algorithm considers:
- Base revenue (most recent earnings)
- Churn rate (cancellation frequency)
- Growth rate (revenue trend)
- Volatility (revenue variance)
- Stream count (diversification)

### 📈 API Endpoints

#### POST `/api/v1/predict/revenue`

Generate revenue predictions for a creator.

**Request:**
```json
{
  "creator_id": "creator_123",
  "include_factors": true
}
```

**Response:**
```json
{
  "creator_id": "creator_123",
  "predictions": [
    {
      "period_days": 30,
      "predicted_revenue": 12500.50,
      "confidence_interval": {
        "lower_bound": 11200.00,
        "upper_bound": 13800.00,
        "confidence_level": 0.95
      },
      "factors": {
        "base_revenue": 10000.00,
        "churn_rate": 0.05,
        "growth_rate": 0.08,
        "volatility": 0.12,
        "stream_count": 15
      }
    },
    {
      "period_days": 60,
      "predicted_revenue": 24800.75,
      "confidence_interval": {
        "lower_bound": 21500.00,
        "upper_bound": 28100.00,
        "confidence_level": 0.95
      },
      "factors": { ... }
    },
    {
      "period_days": 90,
      "predicted_revenue": 37200.25,
      "confidence_interval": {
        "lower_bound": 31000.00,
        "upper_bound": 43400.00,
        "confidence_level": 0.95
      },
      "factors": { ... }
    }
  ],
  "generated_at": "2026-03-26T14:30:00Z"
}
```

#### GET `/api/v1/analytics/{creator_id}/streams`

Get current stream statistics for a creator.

**Response:**
```json
{
  "creator_id": "creator_123",
  "total_streams": 25,
  "total_mrr": 15000.00,
  "avg_stream_value": 600.00,
  "active_streams": 20,
  "churned_streams": 5
}
```

#### GET `/health`

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

## Setup & Installation

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- OpenSSL

### Database Setup

```bash
# Create database
createdb stellar_analytics

# Run migrations
sqlx migrate run --database-url postgres://localhost/stellar_analytics

# Or manually apply schema
psql stellar_analytics < db/schema.sql
```

### Configuration

Create `.env` file:

```env
DATABASE_URL=postgres://user:password@localhost/stellar_analytics
RUST_LOG=info,actix_web=debug
```

### Running the Server

```bash
# Development mode
cargo run

# Release mode
cargo run --release

# Run tests
cargo test
```

## Algorithm Details

### Churn Rate Calculation

```rust
churn_rate = total_cancellations / total_active_streams
```

### Growth Rate (Linear Regression)

Uses log-transformed revenue data for stability:

```rust
slope = Σ((x - x̄)(y - ȳ)) / Σ((x - x̄)²)
growth_rate = e^slope - 1
```

### Monte Carlo Simulation

For each prediction period:
1. Start with base revenue
2. Apply daily net growth (growth - churn)
3. Add random shock from normal distribution
4. Repeat for each day in period
5. Run 1000 simulations
6. Calculate mean and confidence intervals

### Confidence Intervals

Uses percentile method:
- Lower bound: 2.5th percentile
- Upper bound: 97.5th percentile
- Confidence level: 95%

## Data Requirements

### Minimum Data Points

- Requires at least 10 historical data points
- Recommends 30+ days of data for accuracy
- Uses up to 90 days for trend analysis

### Data Quality Checks

- Validates non-negative revenue
- Handles zero-revenue periods gracefully
- Filters outliers beyond 3 standard deviations

## Testing

```bash
# Run unit tests
cargo test predictor::tests

# Run integration tests
cargo test --test integration

# Generate coverage report
cargo tarpaulin --out Html
```

## Performance Benchmarks

| Operation | Latency (p50) | Latency (p99) |
|-----------|---------------|---------------|
| Health Check | < 1ms | < 5ms |
| Revenue Prediction | ~50ms | < 200ms |
| Stream Statistics | ~10ms | < 50ms |

## Future Enhancements

- [ ] Seasonal pattern detection (weekly/monthly cycles)
- [ ] Multi-creator comparative analytics
- [ ] Real-time streaming analytics
- [ ] Machine learning model integration (LSTM, Prophet)
- [ ] Custom prediction periods via API
- [ ] Export predictions as CSV/JSON

## Security Considerations

- All API endpoints should be authenticated in production
- Rate limiting recommended for prediction endpoints
- Database connection pooling configured for security
- Input validation on all creator IDs

## Troubleshooting

**Insufficient data error:**
- Ensure creator has at least 10 days of analytics data
- Check `creator_analytics` table is being populated

**High prediction variance:**
- May indicate volatile revenue streams
- Consider longer historical window for stability
- Review churn rate calculation accuracy

**Slow prediction generation:**
- Reduce Monte Carlo iterations (currently 1000)
- Add caching for repeated requests
- Optimize database queries with proper indexes

---

**Version**: 0.1.0  
**Last Updated**: 2026-03-26
