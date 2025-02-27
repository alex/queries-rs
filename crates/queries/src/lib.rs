use futures::StreamExt;

pub use queries_derive::queries;

pub trait FromRows<DB>: Sized
where
    DB: sqlx::Database,
{
    async fn from_rows<'e>(
        rows: futures::stream::BoxStream<'e, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error>;
}

impl<DB, T> FromRows<DB> for T
where
    DB: sqlx::Database,
    T: for<'a> sqlx::FromRow<'a, DB::Row>,
{
    async fn from_rows<'e>(
        mut rows: futures::stream::BoxStream<'e, Result<DB::Row, sqlx::Error>>,
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

impl<DB, T> FromRows<DB> for Vec<T>
where
    DB: sqlx::Database,
    T: for<'a> sqlx::FromRow<'a, DB::Row>,
{
    async fn from_rows<'e>(
        mut rows: futures::stream::BoxStream<'e, Result<DB::Row, sqlx::Error>>,
    ) -> Result<Self, sqlx::Error> {
        let mut result = vec![];
        while let Some(row) = rows.next().await.transpose()? {
            result.push(T::from_row(&row)?);
        }
        Ok(result)
    }
}
