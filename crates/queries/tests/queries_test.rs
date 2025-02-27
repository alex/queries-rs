#[queries::queries(database = sqlx::Sqlite)]
trait BasicQueries {
    #[query = "SELECT 1"]
    async fn get1() -> (i32,);

    #[query = "SELECT 1 UNION SELECT 2 UNION SELECT 3"]
    async fn get_numbers() -> Vec<(i32,)>;

    #[query = "SELECT ?"]
    async fn get_number(arg: i32) -> (i32,);
}

#[tokio::test]
async fn test_get1() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::new(conn);

    assert_eq!(q.get1().await.unwrap(), (1,));
}

#[tokio::test]
async fn test_get_numbers() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::new(conn);

    assert_eq!(q.get_numbers().await.unwrap(), vec![(1,), (2,), (3,)]);
}

#[tokio::test]
async fn test_get_number() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::new(conn);

    assert_eq!(q.get_number(12).await.unwrap(), (12,));
}
