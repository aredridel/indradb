//! The rocksdb datastore implementation.

mod datastore;
mod managers;

pub use self::datastore::RocksdbDatastore;

#[cfg(feature = "bench-suite")]
full_bench_impl!({
    use super::RocksdbDatastore;
    use tempfile::tempdir;
    let path = tempdir().unwrap().into_path();
    RocksdbDatastore::new(path, Some(1)).unwrap()
});

#[cfg(feature = "test-suite")]
#[cfg(test)]
mod tests {
    #[cfg(feature = "test-suite")]
    full_test_impl!({
        use super::RocksdbDatastore;
        use crate::tests::TestDatabase;
        use tempfile::tempdir;
        let path = tempdir().unwrap().into_path();
        TestDatabase::new(RocksdbDatastore::new(path, Some(1)).unwrap())
    });

    #[test]
    fn should_repair() {
        use super::RocksdbDatastore;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();

        // // Make sure we just initialize the database
        RocksdbDatastore::new(dir.path(), Some(1)).unwrap();

        // Now try to repair
        RocksdbDatastore::repair(dir.path(), Some(1)).unwrap();
    }
}
