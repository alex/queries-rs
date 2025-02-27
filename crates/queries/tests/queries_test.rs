use futures::StreamExt;

#[queries::queries(database = sqlx::Sqlite)]
trait BasicQueries {
    #[query = "SELECT 1"]
    async fn get1() -> (i32,);

    #[query = "SELECT 1 UNION SELECT 2 UNION SELECT 3"]
    async fn get_numbers() -> Vec<(i32,)>;

    #[query = "SELECT ?"]
    async fn get_number(arg: i32) -> (i32,);

    #[query = "SELECT 1 WHERE ?"]
    async fn get1_conditionally(arg: bool) -> Option<(i32,)>;

    #[query = "SELECT 1 UNION SELECT 2 UNION SELECT 3"]
    async fn get_numbers_stream() -> futures::stream::BoxStream<sqlx::Result<(i32,)>>;

    #[query = include_str!("get1.sql")]
    async fn get1_from_file() -> (i32,);
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

#[tokio::test]
async fn test_get1_conditionally() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::new(conn);

    assert_eq!(q.get1_conditionally(true).await.unwrap(), Some((1,)));
    assert_eq!(q.get1_conditionally(false).await.unwrap(), None);
}

#[tokio::test]
async fn test_get_numbers_stream() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::new(conn);

    let mut stream = q.get_numbers_stream().await.unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap(), (1,));
    assert_eq!(stream.next().await.unwrap().unwrap(), (2,));
    assert_eq!(stream.next().await.unwrap().unwrap(), (3,));
}

#[tokio::test]
async fn test_get1_from_file() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::new(conn);

    assert_eq!(q.get1_from_file().await.unwrap(), (1,));
}
