use futures::StreamExt;

#[derive(sqlx::FromRow, Debug, PartialEq)]
struct User {
    id: i32,
    name: String,
}

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

    #[query = "SELECT ?"]
    async fn get_string(arg: &str) -> (String,);

    #[query = "SELECT 1 as id, 'Alex' as name UNION SELECT 2, 'Alice'"]
    async fn get_users() -> Vec<User>;
}

#[tokio::test]
async fn test_get1() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(q.get1().await.unwrap(), (1,));
}

#[tokio::test]
async fn test_get_numbers() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(q.get_numbers().await.unwrap(), vec![(1,), (2,), (3,)]);
}

#[tokio::test]
async fn test_get_number() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(q.get_number(12).await.unwrap(), (12,));
}

#[tokio::test]
async fn test_get1_conditionally() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(q.get1_conditionally(true).await.unwrap(), Some((1,)));
    assert_eq!(q.get1_conditionally(false).await.unwrap(), None);
}

#[tokio::test]
async fn test_get_numbers_stream() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    let mut stream = q.get_numbers_stream().await.unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap(), (1,));
    assert_eq!(stream.next().await.unwrap().unwrap(), (2,));
    assert_eq!(stream.next().await.unwrap().unwrap(), (3,));
}

#[tokio::test]
async fn test_get1_from_file() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(q.get1_from_file().await.unwrap(), (1,));
}

#[tokio::test]
async fn test_get_string() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(q.get_string("abc").await.unwrap(), ("abc".to_string(),));
}

#[tokio::test]
async fn test_get_users() {
    let conn = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(conn);

    assert_eq!(
        q.get_users().await.unwrap(),
        vec![
            User {
                id: 1,
                name: "Alex".to_string()
            },
            User {
                id: 2,
                name: "Alice".to_string()
            }
        ]
    );
}

#[tokio::test]
async fn test_tx() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let q = BasicQueries::from_pool(pool);

    let mut tx_q = q.begin().await.unwrap();
    let result1 = tx_q.get1().await.unwrap();
    let result2 = tx_q.get_number(42).await.unwrap();
    assert_eq!(result1, (1,));
    assert_eq!(result2, (42,));

    tx_q.commit().await.unwrap();
}
