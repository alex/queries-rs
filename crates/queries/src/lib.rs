//! `queries` is a library that makes it easy to manage SQL queries in Rust.
//! Its goal is to provide a straight forward way for developers to write SQL
//! queries and use them in applications (without any risk of SQL injection).
//!
//! It is heavily inspired by [JDBI's SQLobject](https://jdbi.org/#sql-objects).
//!
//! `queries` builds on top of `sqlx`.
//!
//! # Usage
//!
//! The core API is the `#[queries::queries]` proc-macro. For example, you
//! might use it like:
//! ```rust,ignore
//! #[derive(sqlx::FromRow)]
//! struct User {
//!     id: u32,
//!     name: String,
//! }
//!
//! #[queries::queries(database = sqlx::Postgres)]
//! trait MyQueries {
//!     #[query = "SELECT * FROM users WHERE id = $1"]
//!     async fn get_user_by_id(id: u32) -> Option<User>;
//! }
//! ```
//!
//! You can then use `MyQueries` with either a connection pool or transaction:
//! ```rust,ignore
//! // Using a connection pool
//! let connection_pool = sqlx::PgPool::connect("...").await?;
//! let q = MyQueries::from_pool(connection_pool);
//! let user = q.get_user_by_id(42).await?;
//!
//! // Using a transaction
//! let tx = connection_pool.begin().await?;
//! let mut q = MyQueries::from_tx(tx);
//! let user = q.get_user_by_id(42).await?;
//! q.commit().await?; // or q.rollback().await?
//! ```
//!
//! In short, you can declare the signature for each of your queries in a
//! trait, and then make use of them.
//!
//! All return values are automatically wrapped in a `sqlx::Result<>`.
//!
//! # Features
//!
//! `queries` should work with any database supported by `sqlx`.
//!
//! `queries` supports connection pools (`from_pool()`) and transactions
//! (`from_tx()`). Transactions provide `commit()` and `rollback()` methods.
//!
//! Query parameters can use any types that `sqlx` supports (i.e., that
//! implement the `sqlx::Type` and `sqlx::Encode` traits).
//!
//! Query return values can be:
//! - Any type that implements `sqlx::FromRow`.
//! - `Option<T>` (where `T` implements `sqlx::FromRow`)
//! - `Vec<T>` (where `T` implements `sqlx::FromRow`)
//! - `futures::stream::BoxStream<'_, sqlx::Result<T>>` (where `T` implements
//!   `sqlx::FromRow`)
//!
//! For query return types that expect a single row, if the underlying query
//! returns multiple rows then a `sqlx::Decode` error will be returned with an
//! inner error of `queries::MultipleRowsFound`.
//!
//! # Limitations
//!
//! - A given `#[queries::queries]` can only work with a single database (e.g.,
//!   you can't use `MyQueries` with both PostgreSQL and SQLite, you'd need
//!   separate declarations).
//! - All query functions are `async`.

use futures::StreamExt;

pub use queries_derive::queries;

// All this wizard shit is stolen from pyo3
const BASE: u32 = 0;
const VEC: u32 = 1;
const OPTION: u32 = 2;
const STREAM: u32 = 3;
#[doc(hidden)]
pub trait Probe {
    const VALUE: u32 = BASE;
}
#[doc(hidden)]
pub struct FromRowsCategory<T>(std::marker::PhantomData<T>);
impl<T> Probe for FromRowsCategory<T> {}
impl<T> FromRowsCategory<Vec<T>> {
    pub const VALUE: u32 = VEC;
}
impl<T> FromRowsCategory<Option<T>> {
    pub const VALUE: u32 = OPTION;
}
impl<T> FromRowsCategory<futures::stream::BoxStream<'_, T>> {
    pub const VALUE: u32 = STREAM;
}

#[doc(hidden)]
pub trait FromRows<'a, DB, const CATEGORY: u32>: Sized
where
    DB: sqlx::Database,
{
    fn from_rows(
        rows: futures::stream::BoxStream<'a, Result<DB::Row, sqlx::Error>>,
    ) -> impl std::future::Future<Output = Result<Self, sqlx::Error>>;
}

/// An error indicating that a query which expected 1 (or fewer) rows received
/// multiple rows.
#[derive(Debug)]
pub struct MultipleRowsFound;

impl std::fmt::Display for MultipleRowsFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Multiple rows found when at most one was expected")
    }
}

impl std::error::Error for MultipleRowsFound {}

impl<DB, T> FromRows<'_, DB, { BASE }> for T
where
    DB: sqlx::Database,
    T: for<'a> sqlx::FromRow<'a, DB::Row>,
{
    async fn from_rows(
        mut rows: futures::stream::BoxStream<'_, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error> {
        let Some(row) = rows.next().await.transpose()? else {
            return Err(sqlx::Error::RowNotFound);
        };
        if rows.next().await.is_some() {
            return Err(sqlx::Error::Decode(Box::new(MultipleRowsFound)));
        }

        T::from_row(&row)
    }
}

impl<DB, T> FromRows<'_, DB, { OPTION }> for Option<T>
where
    DB: sqlx::Database,
    T: for<'a> sqlx::FromRow<'a, DB::Row>,
{
    async fn from_rows(
        mut rows: futures::stream::BoxStream<'_, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error> {
        let Some(row) = rows.next().await.transpose()? else {
            return Ok(None);
        };
        if rows.next().await.is_some() {
            return Err(sqlx::Error::Decode(Box::new(MultipleRowsFound)));
        }

        Ok(Some(T::from_row(&row)?))
    }
}

impl<DB, T> FromRows<'_, DB, { VEC }> for Vec<T>
where
    DB: sqlx::Database,
    T: for<'a> sqlx::FromRow<'a, DB::Row>,
{
    async fn from_rows(
        mut rows: futures::stream::BoxStream<'_, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error> {
        let mut result = vec![];
        while let Some(row) = rows.next().await.transpose()? {
            result.push(T::from_row(&row)?);
        }
        Ok(result)
    }
}

impl<'a, DB, T> FromRows<'a, DB, { STREAM }> for futures::stream::BoxStream<'a, sqlx::Result<T>>
where
    DB: sqlx::Database,
    T: for<'b> sqlx::FromRow<'b, DB::Row>,
{
    async fn from_rows(
        rows: futures::stream::BoxStream<'a, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error> {
        Ok(rows.map(|r| r.and_then(|row| T::from_row(&row))).boxed())
    }
}
