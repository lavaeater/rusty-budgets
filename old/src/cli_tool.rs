use clap::{App, Arg, SubCommand};
use std::fs;
use std::process::Command;

fn main() {
    let matches = App::new("My Rust Scaffold Tool")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Generates scaffolds for SeaORM and Poem projects")
        .subcommand(
            SubCommand::with_name("generate")
                .about("Generates scaffolds")
                .arg(
                    Arg::with_name("scaffold")
                        .help("The type of scaffold to generate")
                        .required(true)
                        .possible_value("scaffold"),
                )
                .arg(
                    Arg::with_name("name")
                        .help("The name of the model to generate")
                        .required(true),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("generate") {
        let name = matches.value_of("name").unwrap();

        // Call functions to generate the scaffold
        generate_migration(name);
        generate_entity(name);
        generate_crud_handlers(name);
        generate_views(name);
    }
}

fn generate_migration(name: &str) {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let migration_file = format!("migrations/{}_create_{}.rs", timestamp, name);

    // Example content for migration file
    let content = format!(
        r#"
        use sea_orm_migration::prelude::*;

        #[derive(DeriveMigrationName)]
        pub struct Migration;

        #[async_trait::async_trait]
        impl MigrationTrait for Migration {{
            async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
                manager.create_table(
                    Table::create()
                        .table({0})
                        .if_not_exists()
                        .col(ColumnDef::new(Alias::new("id")).integer().not_null().auto_increment().primary_key())
                        .to_owned(),
                )
                .await
            }}

            async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
                manager.drop_table(Table::drop().table({0}).to_owned()).await
            }}
        }}
        "#,
        name
    );

    fs::write(migration_file, content).expect("Failed to write migration file");
    println!("Created migration for {}", name);
}

fn generate_entity(name: &str) {
    Command::new("sea-orm-cli")
        .arg("generate")
        .arg("entity")
        .arg("-o")
        .arg(format!("src/entities/{}", name))
        .status()
        .expect("Failed to generate entity");
    println!("Generated entity for {}", name);
}

fn generate_crud_handlers(name: &str) {
    let handlers_file = format!("src/handlers/{}_handlers.rs", name);
    let content = format!(
        r#"
        use poem::web::Json;

        pub async fn list() -> Json<&'static str> {{
            Json("List of {0}s")
        }}

        pub async fn create() -> Json<&'static str> {{
            Json("Create a {0}")
        }}

        pub async fn update() -> Json<&'static str> {{
            Json("Update a {0}")
        }}

        pub async fn delete() -> Json<&'static str> {{
            Json("Delete a {0}")
        }}
        "#,
        name
    );

    fs::write(handlers_file, content).expect("Failed to write CRUD handlers");
    println!("Created CRUD handlers for {}", name);
}

fn generate_views(name: &str) {
    let views_dir = format!("templates/{}", name);
    fs::create_dir_all(&views_dir).expect("Failed to create views directory");

    let index_file = format!("{}/index.tera", views_dir);
    let index_content = format!(
        r#"
        <h1>List of {0}s</h1>
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Name</th>
                </tr>
            </thead>
            <tbody>
                {{% for {0} in {0}s %}}
                <tr>
                    <td>{{{{ {0}.id }}}}</td>
                    <td>{{{{ {0}.name }}}}</td>
                </tr>
                {{% endfor %}}
            </tbody>
        </table>
        "#,
        name
    );

    fs::write(index_file, index_content).expect("Failed to write index view");
    println!("Created Tera views for {}", name);
}

