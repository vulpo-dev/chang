use sqlx::PgPool;

pub async fn otel(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/otel");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("Otel Migrations done"),
        Err(err) => {
            println!("Failed to run migrations");
            panic!("{:?}", err);
        }
    };
}

pub async fn base(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/base");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("Otel Migrations done"),
        Err(err) => {
            println!("Failed to run migrations");
            panic!("{:?}", err);
        }
    };
}

pub async fn events(pool: &PgPool) {
    let mut migrator = sqlx::migrate!("./migrations/events");
    migrator.set_ignore_missing(true);
    match migrator.run(pool).await {
        Ok(_) => println!("Otel Migrations done"),
        Err(err) => {
            println!("Failed to run migrations");
            panic!("{:?}", err);
        }
    };
}
