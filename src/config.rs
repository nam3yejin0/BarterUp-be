use std::env;
use anyhow::{Context, Result};
use deadpool_postgres::{Config, Pool, Runtime, PoolConfig};
use tokio_postgres::NoTls;

pub fn get_pg_pool() -> Result<Pool> {
    let mut cfg = Config::new();
    cfg.host = Some(env::var("PG_HOST").context("PG_HOST not set")?);
    cfg.user = Some(env::var("PG_USER").context("PG_USER not set")?);
    cfg.password = env::var("PG_PASS").ok();
    cfg.dbname = Some(env::var("PG_DB").context("PG_DB not set")?);
    
    // set pool config safely (PoolConfig.max_size is usize)
    // jika cfg.pool sudah ada, ubah; kalau belum, set default
    if cfg.pool.is_none() {
        cfg.pool = Some(PoolConfig::default());
    }
    if let Some(ref mut pcfg) = cfg.pool {
        pcfg.max_size = 16; // <- tipe: usize (bukan Option)
    }

    // create_pool(runtime, tls)
    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
       .context("failed to create postgres pool")
}
