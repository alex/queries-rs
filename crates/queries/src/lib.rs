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

pub trait FromRows<'a, DB, const CATEGORY: u32>: Sized
where
    DB: sqlx::Database,
{
    async fn from_rows(
        rows: futures::stream::BoxStream<'a, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error>;
}

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
            return todo!();
        }

        Ok(T::from_row(&row)?)
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
            return todo!();
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
