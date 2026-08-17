pub mod api;
pub mod audit;
pub mod embedding_adapter;
pub mod extraction;
pub mod graph;
pub mod model_adapter;
pub mod obligation;
pub mod transcript;

// ADR-0057: backend unit tests share one Postgres server with the running dev
// stack. This guard makes test-database isolation enforced, not merely a
// documented convention (ADR-0056): every test_pool() calls it before
// connecting, and it panics unless DATABASE_URL targets the isolated
// ringmaster_test database -- so a stray run against the long-lived ringmaster
// database a person actually reads fails loudly instead of silently polluting it.
#[cfg(test)]
pub(crate) fn guard_test_database(database_url: &str) {
    const ISOLATED_DATABASE: &str = "ringmaster_test";
    let database_name = database_url
        .rsplit('/')
        .next()
        .map(|tail| tail.split(['?', '#']).next().unwrap_or(tail))
        .unwrap_or("");
    assert_eq!(
        database_name, ISOLATED_DATABASE,
        "refusing to run backend tests against database {database_name:?} \
         (DATABASE_URL={database_url:?}); tests must target {ISOLATED_DATABASE:?} \
         -- see ADR-0057 and docs/CONTRIBUTING.md"
    );
}

#[cfg(test)]
mod guard_tests {
    use super::guard_test_database;

    #[test]
    fn guard_accepts_isolated_database() {
        guard_test_database("postgres://ringmaster:ringmaster-dev@postgres:5432/ringmaster_test");
        guard_test_database("postgres://u:p@localhost:5432/ringmaster_test?sslmode=require");
    }

    #[test]
    #[should_panic(expected = "refusing to run backend tests")]
    fn guard_rejects_dev_database() {
        guard_test_database("postgres://ringmaster:ringmaster-dev@postgres:5432/ringmaster");
    }
}
