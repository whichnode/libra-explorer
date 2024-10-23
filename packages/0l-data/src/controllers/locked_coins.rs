use std::{
    sync::Arc,
    vec,
};
use arrow_array::{
    FixedSizeBinaryArray, RecordBatch, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};
use axum::{http::StatusCode, response::IntoResponse, Extension};
use datafusion::{
    dataframe::DataFrame,
    datasource::MemTable,
    execution::context::SessionContext,
    logical_expr::{coalesce, col, lit, JoinType},
    scalar::ScalarValue,
    sql::TableReference,
};
use ethereum_types::U256;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::app_state::AppState;

async fn get_slow_wallet_balances(app_state: &AppState) -> MemTable {
    let params = &[("database", app_state.clickhouse_database.clone())];

    let qs = serde_urlencoded::to_string(params).unwrap();

    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/?{}", app_state.clickhouse_host.clone(), qs))
        .header("X-ClickHouse-User", app_state.clickhouse_username.clone())
        .header("X-ClickHouse-Key", app_state.clickhouse_password.clone())
        .body(
            r#"
                SELECT
                    tupleElement("entry", 2) AS "version",
                    toUInt64(tupleElement("entry", 3)) AS "value",
                    tupleElement("entry", 4) AS "address"
                FROM (
                    SELECT
                        arrayElement(
                            arraySort(
                                (x) -> tupleElement(x, 1),
                                groupArray(
                                    tuple(
                                        "change_index",
                                        "version",
                                        "balance",
                                        "address"
                                    )
                                )
                            ),
                            -1
                        ) AS "entry"
                    FROM
                        "coin_balance"
                    WHERE
                        "address" IN (
                            SELECT DISTINCT "address"
                            FROM "slow_wallet"
                        )
                    GROUP BY
                        "address",
                        "version"
                    ORDER BY
                        "address",
                        "version" ASC
                )
                FORMAT Parquet
            "#,
        )
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let parquet_reader = ParquetRecordBatchReaderBuilder::try_new(res)
        .unwrap()
        .with_batch_size(8192)
        .build()
        .unwrap();

    let batches = parquet_reader
        .into_iter()
        .map(|it| it.unwrap())
        .collect::<Vec<RecordBatch>>();

    let schema = Schema::new(vec![
        Field::new("version", DataType::UInt64, false),
        Field::new("value", DataType::UInt64, false),
        Field::new("address", DataType::FixedSizeBinary(32), false),
    ]);

    MemTable::try_new(Arc::new(schema), vec![batches]).unwrap()
}

async fn get_slow_wallet(app_state: &AppState) -> MemTable {
    let params = &[("database", app_state.clickhouse_database.clone())];

    let qs = serde_urlencoded::to_string(params).unwrap();

    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/?{}", app_state.clickhouse_host.clone(), qs,))
        .header("X-ClickHouse-User", app_state.clickhouse_username.clone())
        .header("X-ClickHouse-Key", app_state.clickhouse_password.clone())
        .body(
            r#"
                SELECT
                    toUInt64(tupleElement("entry", 2)) AS "unlocked",
                    tupleElement("entry", 3) AS "version",
                    tupleElement("entry", 4) AS "address"
                FROM (
                    SELECT
                        arrayElement(
                            arraySort(
                                (x) -> tupleElement(x, 1),
                                groupArray(
                                    tuple(
                                        "change_index",
                                        "unlocked",
                                        "version",
                                        "address"
                                    )
                                )
                            ),
                            -1
                        ) AS "entry"
                    FROM "slow_wallet"
                    GROUP BY
                      "address",
                      "version"
                    ORDER BY
                      "address",
                      "version" ASC
                )
                FORMAT Parquet
            "#,
        )
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let parquet_reader = ParquetRecordBatchReaderBuilder::try_new(res)
        .unwrap()
        .with_batch_size(8192)
        .build()
        .unwrap();

    let batches = parquet_reader
        .into_iter()
        .map(|it| it.unwrap())
        .collect::<Vec<RecordBatch>>();

    let schema = Schema::new(vec![
        Field::new("unlocked", DataType::UInt64, false),
        Field::new("version", DataType::UInt64, false),
        Field::new("address", DataType::FixedSizeBinary(32), false),
    ]);

    MemTable::try_new(Arc::new(schema), vec![batches]).unwrap()
}

async fn get_locked_hist(hist: DataFrame) -> Vec<(u64, u64)> {
    let mut unlocked_mem: Option<u64> = None;
    let mut balance_mem = 0;

    hist.collect().await.unwrap().iter().map(|batch| {
        let version = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Failed to downcast time")
            .values()
            .to_vec();

        let balance_col = batch
            .column(1)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Failed to downcast unlocked")
            .into_iter()
            .collect::<Vec<Option<u64>>>();

        let unlocked_col = batch
            .column(2)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("Failed to downcast unlocked")
            .into_iter()
            .collect::<Vec<Option<u64>>>();

        balance_col.iter().enumerate().map(|(index, balance)| {
            let balance = balance.unwrap_or(balance_mem);
            balance_mem = balance;

            let unlocked = if let Some(unlocked_value) = unlocked_col[index] {
                unlocked_mem = Some(unlocked_value);
                unlocked_value
            } else if let Some(unlocked_mem) = unlocked_mem {
                unlocked_mem
            } else {
                balance_mem
            };

            if unlocked <= balance {
                (version[index], balance - unlocked)
            } else {
                (version[index], 0)
            }
        }).collect::<Vec<(u64, u64)>>()
    }).flatten().collect()
}

pub async fn get(Extension(app_state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let balance = get_slow_wallet_balances(&app_state).await;
    let slow_wallet = get_slow_wallet(&app_state).await;

    let ctx = SessionContext::new();

    ctx.register_table(
        TableReference::Bare {
            table: "slow_wallet".into(),
        },
        Arc::new(slow_wallet),
    )
    .unwrap();

    ctx.register_table(
        TableReference::Bare {
            table: "balance".into(),
        },
        Arc::new(balance),
    )
    .unwrap();

    let balance = ctx.table("balance").await.unwrap();
    let slow_wallet = ctx.table("slow_wallet").await.unwrap();

    let timestamps = slow_wallet
        .join(
            balance,
            JoinType::Full,
            &["version"],
            &["version"],
            None
        )
        .unwrap()
        .select(
            vec![
                coalesce(
                    vec![
                        col("slow_wallet.version"),
                        col("balance.version"),
                    ]
                )
                .alias("version")
            ]
        )
        .unwrap()
        .distinct()
        .unwrap()
        .sort(vec![col("version").sort(true, true)])
        .unwrap();

    let schema = Schema::new(vec![
        Field::new("version", DataType::UInt64, true)
    ]);
    let versions = MemTable::try_new(
        Arc::new(schema),
        timestamps.collect_partitioned().await.unwrap(),
    )
    .unwrap();

    ctx.register_table(
        TableReference::Bare {
            table: "versions".into(),
        },
        Arc::new(versions),
    )
    .unwrap();

    let versions: Vec<u64> = ctx.table("versions").await.unwrap()
        .collect()
        .await
        .unwrap()
        .iter()
        .map(|batch| {
            batch
                .column(0)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .expect("Failed to downcast time")
                .values()
                .to_vec()
        })
        .flatten()
        .collect();

    let len = versions.len();
    let mut total = vec![0f64; len];

    let df = ctx
        .sql("SELECT DISTINCT address FROM slow_wallet")
        .await
        .unwrap();

    let addresses = df
        .collect()
        .await
        .unwrap()
        .iter()
        .map(|batch| {
            let addresses = batch
                .column(0)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .expect("Failed to downcast time")
                .values()
                .to_vec();
            let (_, addresses, _) = unsafe { addresses.align_to::<U256>() };
            addresses
                .iter()
                .map(|addr| addr.clone())
                .collect::<Vec<U256>>()
        })
        .flatten()
        .collect::<Vec<U256>>();

    for address in addresses.into_iter() {
        let addr = Into::<[u8; 32]>::into(address);
        let mut addr = Vec::from(addr);
        addr.reverse();
        let addr = ScalarValue::FixedSizeBinary(32, Some(addr));

        let balance = ctx
            .table("balance")
            .await
            .unwrap()
            .filter(col("address").eq(lit(addr.clone())))
            .unwrap();

        let slow_wallet = ctx
            .table("slow_wallet")
            .await
            .unwrap()
            .filter(col("slow_wallet.address").eq(lit(addr.clone())))
            .unwrap();

        let hist = slow_wallet
            .join(
                balance,
                JoinType::Full,
                &["version"],
                &["version"],
                None
            )
            .unwrap()
            .select(vec![
                coalesce(vec![
                    col("slow_wallet.version"),
                    col("balance.version")
                ]).alias("version"),
                col("balance.value").alias("balance"),
                col("slow_wallet.unlocked"),
            ])
            .unwrap()
            .sort(vec![col("version").sort(true, true)])
            .unwrap();

        let mut i = 0;
        let mut prev = 0f64;

        let locked_hist = get_locked_hist(hist).await;
        for (version, locked) in locked_hist {
            while versions[i] < version {
                total[i] += prev;
                i += 1;
            }

            if versions[i] == version {
                prev = (locked as f64) / 1e6;
                total[i] += prev;
                i += 1;
            }
        }

        while i < versions.len() {
            total[i] += prev;
            i += 1;
        }
    }

    let mut locked_coins = Vec::with_capacity(versions.len());
    for i in 0..versions.len() {
        locked_coins.push((versions[i], total[i]));
    }

    axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&&locked_coins)
            .unwrap(),
        )
        .unwrap()
}
