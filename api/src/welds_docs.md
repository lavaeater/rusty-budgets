# Welds Docs

## CRUD

Crud from the Welds Github page

```rust
use welds::{Syntax, WeldsError, prelude::*};

/// Define a struct the maps to the products table in the databases
#[derive(Debug, WeldsModel)]
#[welds(table = "products")]
#[welds(HasMany(orders, Order, "product_id"))]
pub struct Product {
    #[welds(primary_key)]
    #[welds(rename = "product_id")]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    #[welds(rename = "price1")]
    pub price: Option<f32>,
    pub active: bool,
}

/// Define a Struct the maps to the Orders table in the databases
#[derive(Debug, WeldsModel)]
#[welds(table = "orders")]
#[welds(BelongsTo(product, Product, "product_id"))]
pub struct Order {
    #[welds(primary_key)]
    pub id: i32,
    pub product_id: Option<i32>,
    #[welds(rename = "price")]
    pub sell_price: Option<f32>,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let connection_string = "sqlite::memory:";
    let client = welds::connections::connect(connection_string).await?;

    // Build an in memory DB with a schema (Product Table, Orders Table)
    let schema = include_str!("../../tests/testlib/databases/sqlite/01_create_tables.sql");
    client.execute(schema, &[]).await?;

    // Create and update a Product
    let trans = client.begin().await?;
    let product = create_and_update_products(&trans).await?;
    trans.commit().await?;

    // Create a bunch of orders
    create_orders(&product, &client).await?;

    // Select the Orders Using the Product
    chain_query_together(&client).await?;

    // Filter Orders using relationships from other tables
    filter_order_using_relationships(&client).await?;

    // Delete Some Stuff
    let product2 = create_and_update_products(&client).await?;
    delete_the_product(&client, product2.id).await?;

    let _ = Product::all().set(|x| x.description, "".to_string());

    Ok(())
}

async fn create_and_update_products(client: &impl Client) -> Result<DbState<Product>, WeldsError> {
    // create the product
    let mut p = Product::new();
    p.name = "Girl Scout Cookies".to_owned();
    p.active = true;
    p.save(client).await?;
    println!("Product Created: {:?}", p);

    // update the product
    p.description = Some("Yummy !!!".to_owned());
    p.save(client).await?;
    println!("Product Updated: {:?}", p);
    Ok(p)
}

async fn create_orders(product: &Product, conn: &impl Client) -> Result<(), WeldsError> {
    for _ in 0..100 {
        let mut o = Order::new();
        o.product_id = Some(product.id);
        o.sell_price = Some(3.50);
        o.save(conn).await?;
    }
    let total = Order::all().count(conn).await?;
    println!();
    println!("Orders Created: {}", total);
    Ok(())
}

async fn chain_query_together(conn: &impl Client) -> Result<(), WeldsError> {
    // Start from a product and ending on its orders
    let order_query = Product::all()
        .order_by_asc(|p| p.id)
        .limit(1)
        .map_query(|p| p.orders)
        .where_col(|x| x.id.lte(2));

    let sql = order_query.to_sql(Syntax::Sqlite);

    let orders = order_query.run(conn).await?;

    println!();
    println!("Some Orders SQL: {}", sql);
    println!("Some Orders: {:?}", orders);

    Ok(())
}

async fn filter_order_using_relationships(
    conn: &impl Client,
) -> Result<(), Box<dyn std::error::Error>> {
    // NOTE: this is an un-executed query.
    let product_query = Product::where_col(|p| p.name.like("%Cookie%"));

    // select all the orders, where order match the product query
    let orders = Order::all()
        .where_relation(|o| o.product, product_query)
        .run(conn)
        .await?;

    println!();
    println!("Found More Orders: {}", orders.len());
    Ok(())
}

async fn delete_the_product(
    conn: &impl Client,
    product_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut product = Product::find_by_id(conn, product_id).await?.unwrap();
    product.delete(conn).await?;
    let count = Product::all().count(conn).await?;

    println!();
    println!("DELETE: {:?}", product);
    println!("NEW COUNT: {}", count);
    Ok(())
}
```

## Migrations

This is from Welds Github:

```rust

use welds::errors::Result;
use welds::migrations::prelude::*;

#[async_std::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    // Connect and setup a DB for use to play with
    let client = welds::connections::connect("sqlite::memory:").await?;

    // run all the migrations
    // This will skip over migrations that have already ran
    up(
        &client,
        &[
            create_peoples_table,
            create_addresses_table,
            add_address_to_people,
            rename_and_make_nullable,
        ],
    )
    .await?;
    println!("Migrate Up Complete");

    // lets rollback the last change
    let downed = down_last(&client).await?;
    println!("Migrate Down Complete");
    println!("Rollback: {}", downed.unwrap());

    Ok(())
}

// A simple migration to setup the peoples table.
fn create_peoples_table(_: &TableState) -> Result<MigrationStep> {
    let m = create_table("people")
        .id(|c| c("id", Type::Int))
        .column(|c| c("name", Type::String).create_unique_index());
    Ok(MigrationStep::new("create_peoples_table", m))
}

// A simple migration to setup the addresses table.
fn create_addresses_table(_: &TableState) -> Result<MigrationStep> {
    let m = create_table("addresses")
        .id(|c| c("id", Type::Int))
        .column(|c| c("name", Type::String).create_unique_index())
        .column(|c| c("finger_count", Type::IntSmall));
    Ok(MigrationStep::new("create_addresses_table", m))
}

// Let add a column to people to wire the two together
fn add_address_to_people(state: &TableState) -> Result<MigrationStep> {
    let alter = change_table(state, "people")?;
    let m = alter.add_column("aaddress_id", Type::Int);
    Ok(MigrationStep::new("add_address_to_people", m))
}

// Let add a column to people to wire the two together
fn rename_and_make_nullable(state: &TableState) -> Result<MigrationStep> {
    // fix the bad spelling :)
    let alter = change_table(state, "people")?;
    let m = alter.change("aaddress_id").null().rename("address_id");
    Ok(MigrationStep::new("rename_and_make_nullable", m))
}
```

## Hooks

Also from the welds github

```rust
use welds::prelude::*;

/// Define a struct the maps to the products table in the databases
#[derive(Debug, WeldsModel)]
#[welds(table = "products")]
// Wiring up a bunch of hooks for when this model touches the database.
#[welds(BeforeCreate(before_create))]
#[welds(AfterCreate(after_create, async = true))]
#[welds(AfterCreate(after_create_second))]
#[welds(BeforeUpdate(before_update))]
#[welds(AfterUpdate(after_update))]
#[welds(BeforeDelete(before_delete))]
#[welds(AfterDelete(after_delete))]

pub struct Product {
    #[welds(primary_key)]
    #[welds(rename = "product_id")]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    #[welds(rename = "price1")]
    pub price: Option<f32>,
    pub active: bool,
}

// *******************************************************************
// Note: welds::errors::WeldsError does support Anyhow errors
// This way your own types can be passed through
// *******************************************************************

fn before_create(product: &mut Product) -> welds::errors::Result<()> {
    println!("Before Create: {:?}", product);
    Ok(())
}

// Example async callback
async fn after_create(product: &Product) {
    print_message(product).await;
}

fn after_create_second(product: &Product) {
    // they run in the order they are defined
    println!("After Create2: {:?}", product);
}

fn before_update(product: &mut Product) -> welds::errors::Result<()> {
    println!("Before Update: {:?}", product);
    Ok(())
}

fn after_update(product: &Product) {
    println!("After Update: {:?}", product);
}

fn before_delete(product: &Product) -> welds::errors::Result<()> {
    eprintln!("Before Delete: {:?}", product);
    Ok(())
}

fn after_delete(product: &Product) {
    println!("After Delete: {:?}", product);
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let connection_string = "sqlite::memory:";
    let client = welds::connections::connect(connection_string).await?;

    // Build an in memory DB with a schema (Product Table, Orders Table)
    let schema = include_str!("../../tests/testlib/databases/sqlite/01_create_tables.sql");
    client.execute(schema, &[]).await?;

    let mut product = new_product();
    product.save(&client).await?;

    product.name = "test".to_string();
    product.save(&client).await?;

    product.delete(&client).await?;

    eprintln!("Done");

    Ok(())
}

fn new_product() -> DbState<Product> {
    DbState::new_uncreated(Product {
        id: 0,
        name: "Cookie".to_owned(),
        description: None,
        price: Some(3.15),
        active: true,
    })
}

async fn print_message(product: &Product) {
    println!("After Create: {:?}", product);
}
```