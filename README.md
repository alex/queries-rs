# `queries-rs`

This library is heavily inspired by [JDBI's SQLobject](https://jdbi.org/#sql-objects).

The goal of this library is to provide a straight forward way for developers to write SQL queries and use them in applications (without any risk of SQL injection).

A basic example of how to declare queries looks like:

```rust

#[queries::queries(database = sqlx::Sqlite)]
trait MyQueries {
    #[query = "SELECT 1"]
    async fn get1() -> (i32,);

    #[query = "SELECT name FROM people"]
    async fn get_names() -> Vec<(String,)>;

    #[query = "SELECT name, age FROM people WHERE id = $1"]
    async fn get_name_age_by_id(id: u32) -> (String, u32);
}
```

And then to use them:

```rust
let connection_pool = todo!()
let q = MyQueries::new(connection_pool);

let (one,) = q.get1().await?;
assert_eq!(one, 1);

let (name, age) = q.get_name_age_by_id(1).await?;
```

## Implementation Notes

- Right now all functions generated are `async`.
- Functions don't need to be annotated as returning a `Result<>`, that's done
  automatically.
- Even though the user write `trait`, the generated code is actually a `struct`
  which provides the documented APIs.
- Functions can return either a single row (any type that implements
  `sqlx::FromRow`), a `Vec<>` of rows, or an `futures::Stream<>` of rows.
  - If a query returns a single row, an error will be returned if the query
    does not return exactly one row from the database.
  - Queries can return `Option<T>` in which case `None` will be returned if
    there are no rows. (An error will still be returned if multiple rows are
    returned from the database).
  - This is done using the `queries::FromRows` trait.
- Arguments can be any types that implement `sqlx::Encode` and `sqlx::Type`.
