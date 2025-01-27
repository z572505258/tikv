// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use criterion::black_box;

use tipb::executor::Aggregation;
use tipb::expression::Expr;

use tidb_query::batch::executors::BatchFastHashAggregationExecutor;
use tidb_query::batch::executors::BatchSlowHashAggregationExecutor;
use tidb_query::batch::interface::*;
use tidb_query::executor::{Executor, HashAggExecutor};
use tidb_query::expr::EvalConfig;
use tikv::storage::Statistics;

use crate::util::bencher::Bencher;
use crate::util::executor_descriptor::hash_aggregate;
use crate::util::FixtureBuilder;

pub trait HashAggrBencher {
    fn name(&self) -> &'static str;

    fn bench(
        &self,
        b: &mut criterion::Bencher,
        fb: &FixtureBuilder,
        group_by_expr: &[Expr],
        aggr_expr: &[Expr],
    );

    fn box_clone(&self) -> Box<dyn HashAggrBencher>;
}

impl Clone for Box<dyn HashAggrBencher> {
    #[inline]
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// A bencher that will use normal hash aggregation executor to bench the giving aggregate
/// expression.
pub struct NormalBencher;

impl HashAggrBencher for NormalBencher {
    fn name(&self) -> &'static str {
        "normal"
    }

    fn bench(
        &self,
        b: &mut criterion::Bencher,
        fb: &FixtureBuilder,
        group_by_expr: &[Expr],
        aggr_expr: &[Expr],
    ) {
        crate::util::bencher::NormalNextAllBencher::new(|| {
            let meta = hash_aggregate(aggr_expr, group_by_expr).take_aggregation();
            let src = fb.clone().build_normal_fixture_executor();
            let ex = HashAggExecutor::new(
                black_box(meta),
                black_box(Arc::new(EvalConfig::default())),
                black_box(Box::new(src)),
            )
            .unwrap();
            Box::new(ex) as Box<dyn Executor<StorageStats = Statistics>>
        })
        .bench(b);
    }

    fn box_clone(&self) -> Box<dyn HashAggrBencher> {
        Box::new(Self)
    }
}

/// A bencher that will use batch hash aggregation executor to bench the giving aggregate
/// expression.
pub struct BatchBencher;

impl HashAggrBencher for BatchBencher {
    fn name(&self) -> &'static str {
        "batch"
    }

    fn bench(
        &self,
        b: &mut criterion::Bencher,
        fb: &FixtureBuilder,
        group_by_expr: &[Expr],
        aggr_expr: &[Expr],
    ) {
        crate::util::bencher::BatchNextAllBencher::new(|| {
            let src = fb.clone().build_batch_fixture_executor();
            let mut meta = Aggregation::default();
            meta.set_agg_func(aggr_expr.to_vec().into());
            meta.set_group_by(group_by_expr.to_vec().into());
            if BatchFastHashAggregationExecutor::check_supported(&meta).is_ok() {
                let ex = BatchFastHashAggregationExecutor::new(
                    black_box(Arc::new(EvalConfig::default())),
                    black_box(Box::new(src)),
                    black_box(group_by_expr.to_vec()),
                    black_box(aggr_expr.to_vec()),
                )
                .unwrap();
                Box::new(ex) as Box<dyn BatchExecutor<StorageStats = Statistics>>
            } else {
                let ex = BatchSlowHashAggregationExecutor::new(
                    black_box(Arc::new(EvalConfig::default())),
                    black_box(Box::new(src)),
                    black_box(group_by_expr.to_vec()),
                    black_box(aggr_expr.to_vec()),
                )
                .unwrap();
                Box::new(ex) as Box<dyn BatchExecutor<StorageStats = Statistics>>
            }
        })
        .bench(b);
    }

    fn box_clone(&self) -> Box<dyn HashAggrBencher> {
        Box::new(Self)
    }
}
